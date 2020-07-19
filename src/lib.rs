//! This repo is an ergonomics experiment on a way to temporarily and reversibly modify data structures in Rust. The idea is
//! that it lets you hand a data structure off to another piece of code saying "okay, you can modify this data while you're
//! using it, but you have to put it back to the way you found it." The approach here gives each mutating operation a scope,
//! reverting the operation when it goes out of scope. So once the operation goes out of scope, the state of the data
//! structure will be returned to what it was before the scoped operation was applied (except for maybe like the capacity of
//! a `Vec` could be different, or something like that). Because each operation is reverted when it goes out of scope,
//! operations can be nested without complication. Conceptually this is like a weaker version of a partially persistent data
//! structure.
//!
//! Here's a simple example of pushing an element onto a Vec, and having the change automatically
//! undone:
//!
//! ```
//! use scoped_ops::VecScoped;
//!
//! let mut a = vec![1];
//! {
//!     let mut b = a.pushed(2);
//!     assert_eq!([1, 2], *b);
//! }  // b drops, and undoes its change
//!
//! assert_eq!([1], *a);
//! ```
//!
//! Advantages:
//!
//! - Uses normal Rust data structures, so can be integrated into existing code
//! - Hopefully zero-cost abstraction, although inspection would be needed to verify
//! - No dependencies, could work without `std`
//!
//! Disadvantages:
//!
//! - These reversions could in many cases just be coded by hand
//! - Possibly slow compilation
//! - The generics are kind of a beast; you'll end up with complex nested types like when using futures or iterators
//! - The syntax is a bit long and indent-y for my tastes
//!
//! I can think of a few possible alternatives for temporarily and reversibly modifying data:
//!
//! - Just clone the data, then you can modify the clone however you want
//! - Trust the user to take care of it
//! - Apply adapters around the original data structure at read time. For example, you could have a `VecPushed` that has
//!   most of the same methods as `Vec`, but acts as if it has an element pushed onto the end. For example, if the
//!   underlying `Vec` has length 2, the `VecPushed` would say that its length is 3. Possible disadvantages:
//!   - A lot of code to implement
//!   - Applying a lot of modifications, or certain kinds of modifications, might hurt performance
//!   - You couldn't get a slice, because the data doesn't actually exist in memory
//!
//! To do:
//!
//! - Explore owned variant
//! - Explore support for "commit vs. revert"
//! - Add support for other data structures?
//! - Explore reference-counted variant?
//! - Figure out if this would actually be useful for anything 😂?

/// Everything that is `VecScoped` will need to have mutable access to the underlying `Vec`.
/// However, only the trait implementations should be allowed to mutate the `Vec`; end users
/// should not, because they could violate an invariant.
pub trait VecScopedPrivate {
    type Element;

    fn vec_mut(&mut self) -> &mut Vec<Self::Element>;
}

use std::ops::Deref;

/// This trait represent a `Vec` or a temporary modification of a `Vec`
pub trait VecScoped<T>: VecScopedPrivate<Element = T> {
    /// Temporarily push an element onto the end of the `Vec`
    fn pushed(&mut self, value: T) -> Push<Self>
    where
        Self: Sized,
    {
        Push::new(self, value)
    }

    /// Temporarily pop the last element from the end of the `Vec`
    fn popped(&mut self) -> Pop<Self>
    where
        Self: Sized,
    {
        Pop::new(self)
    }

    /// Temporarily assign an element at `idx` of the `Vec`.
    /// Panics if `idx` is out of bounds.
    fn assigned(&mut self, idx: usize, value: T) -> Assign<Self>
    where
        Self: Sized,
    {
        Assign::new(self, value, idx)
    }
}

impl<T> VecScopedPrivate for Vec<T> {
    type Element = T;

    fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
        self
    }
}

impl<T> VecScoped<T> for Vec<T> {}

// TODO would users ever want to access the element that was popped? or if an element was popped?
/// See `crate::VecScoped::pop`
#[must_use]
pub struct Pop<'a, V: VecScopedPrivate> {
    inner: &'a mut V,
    popped: Option<V::Element>,
}

impl<'a, V: VecScopedPrivate> Pop<'a, V> {
    pub fn new(inner: &'a mut V) -> Self {
        let popped = inner.vec_mut().pop();
        Self { inner, popped }
    }
}

impl<'a, T, V: Deref<Target = [T]> + VecScopedPrivate> Deref for Pop<'a, V> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, V: VecScopedPrivate> Drop for Pop<'a, V> {
    fn drop(&mut self) {
        if let Some(popped) = self.popped.take() {
            self.vec_mut().push(popped)
        }
    }
}

impl<'a, V: VecScopedPrivate> VecScopedPrivate for Pop<'a, V> {
    type Element = V::Element;

    fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
        self.inner.vec_mut()
    }
}

impl<'a, T, V: VecScopedPrivate<Element = T>> VecScoped<T> for Pop<'a, V> {}

// TODO would users ever want to access the element that was popped?
/// See `crate::VecScoped::push`
#[must_use]
pub struct Push<'a, V: VecScopedPrivate>(&'a mut V);

impl<'a, V: VecScopedPrivate> Push<'a, V> {
    pub fn new(vec_scoped: &'a mut V, value: V::Element) -> Self {
        vec_scoped.vec_mut().push(value);
        Self(vec_scoped)
    }
}

impl<'a, T, V: Deref<Target = [T]> + VecScopedPrivate> Deref for Push<'a, V> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, V: VecScopedPrivate> Drop for Push<'a, V> {
    fn drop(&mut self) {
        let _did_pop = self.0.vec_mut().pop().is_some();
        debug_assert!(_did_pop, "Someone has illicitly popped an element!");
    }
}

impl<'a, V: VecScopedPrivate> VecScopedPrivate for Push<'a, V> {
    type Element = V::Element;

    fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
        self.0.vec_mut()
    }
}

impl<'a, T, V: VecScopedPrivate<Element = T>> VecScoped<T> for Push<'a, V> {}

pub struct Assign<'a, V: VecScopedPrivate> {
    inner: &'a mut V,
    idx: usize,
    previous: V::Element,
}

impl<'a, V: VecScopedPrivate> Assign<'a, V> {
    pub fn new(vec_scoped: &'a mut V, mut value: V::Element, idx: usize) -> Self {
        let inner = vec_scoped.vec_mut();
        if let Some(old) = inner.get_mut(idx) {
            std::mem::swap(old, &mut value);
        } else {
            panic!(
                "assigned index (is {}) should be < len (is {})",
                idx,
                inner.len()
            )
        }
        Self {
            inner: vec_scoped,
            idx,
            previous: value,
        }
    }
}

impl<'a, T, V: Deref<Target = [T]> + VecScopedPrivate> Deref for Assign<'a, V> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, V: VecScopedPrivate> Drop for Assign<'a, V> {
    fn drop(&mut self) {
        let idx = self.idx;
        let inner = self.inner.vec_mut();
        if let Some(old) = inner.get_mut(idx) {
            std::mem::swap(old, &mut self.previous);
        } else {
            panic!(
                "dropping assigned index (is {}) should be < len (is {}), this should never happen",
                idx,
                inner.len()
            )
        }
    }
}

impl<'a, V: VecScopedPrivate> VecScopedPrivate for Assign<'a, V> {
    type Element = V::Element;

    fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
        self.inner.vec_mut()
    }
}

impl<'a, T, V: VecScopedPrivate<Element = T>> VecScoped<T> for Assign<'a, V> {}

#[test]
fn test_scoped_vec() {
    let mut a = vec![1];
    {
        let mut b = a.pushed(2);
        {
            assert_eq!([1, 2, 3], *b.pushed(3));
        }
        assert_eq!([1, 2, -3], *b.pushed(-3));
    }
    assert_eq!([1, -2], *a.pushed(-2));
    assert_eq!([1], *a);
}

#[test]
fn test_pop_empty() {
    let mut a = Vec::<i32>::new();
    {
        assert_eq!([0i32; 0], *a.popped());
    }
    assert_eq!([0i32; 0], *a);
}

#[test]
fn test_pop() {
    let mut a = vec![1];
    {
        assert_eq!([0i32; 0], *a.popped());
    }
    assert_eq!([1], *a);
}

#[test]
fn test_pop_push() {
    let mut a = vec![1];
    {
        assert_eq!([-1], *a.popped().pushed(-1));
    }
    assert_eq!([1], *a);
}

#[test]
fn test_assigned() {
    let mut a = vec![0, 1, 2, 3];
    {
        assert_eq!([0, 1, 5, 3], *a.assigned(2, 5))
    }
    assert_eq!([0, 1, 2, 3], *a);
}

#[test]
#[should_panic]
fn test_assigned_panics_with_out_of_bounds_index() {
    vec![1].assigned(2, 5);
}

// TODO automatically verify that this warns
#[test]
fn test_must_use() {
    let mut a = vec![1];
    a.pushed(2); // This pushes a value that is then immediately popped, which is useless
    assert_eq!([1], *a);
}

pub mod owned {
    pub trait VecScopedPrivate {
        type Element;

        fn vec_mut(&mut self) -> &mut Vec<Self::Element>;
    }

    use std::ops::Deref;

    /// This trait represent a `Vec` or a temporary modification of a `Vec`
    pub trait VecScoped<T>: VecScopedPrivate<Element = T> {
        /// Temporarily pop the last element from the end of the `Vec`
        fn popped(self) -> Pop<Self>
            where
                Self: Sized,
        {
            Pop::new(self)
        }
    }

    impl<T> VecScopedPrivate for Vec<T> {
        type Element = T;

        fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
            self
        }
    }

    impl<T> VecScoped<T> for Vec<T> {}

    // TODO would users ever want to access the element that was popped? or if an element was popped?
    /// See `crate::VecScoped::pop`
    #[must_use]
    pub struct Pop<V: VecScopedPrivate> {
        inner: V,
        popped: Option<V::Element>,
    }

    impl<V: VecScopedPrivate> Pop<V> {
        pub fn new(mut inner: V) -> Self {
            let popped = inner.vec_mut().pop();
            Self { inner, popped }
        }

        pub fn into_inner(mut self) -> V {
            if let Some(popped) = self.popped.take() {
                self.vec_mut().push(popped)
            }
            self.inner
        }
    }

    impl<T, V: Deref<Target = [T]> + VecScopedPrivate> Deref for Pop<V> {
        type Target = [T];

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl<V: VecScopedPrivate> VecScopedPrivate for Pop<V> {
        type Element = V::Element;

        fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
            self.inner.vec_mut()
        }
    }

    impl<T, V: VecScopedPrivate<Element = T>> VecScoped<T> for Pop<V> {}

    #[test]
    fn test_pop() {
        let a = vec![1];
        let b = a.popped();
        assert_eq!([0i32; 0], *b);
        assert_eq!([1], *b.into_inner());
    }
}
