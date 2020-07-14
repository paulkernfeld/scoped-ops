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
mod private {
    // TODO is this a bad idea, because it would prevent users from implementing their own ops?
    /// Everything that is `VecScoped` will need to have mutable access to the underlying `Vec`.
    /// However, only the trait implementations should be allowed to mutate the `Vec`; end users
    /// should not, because they could violate an invariant.
    ///
    /// It's illegal to have a private trait in a public interface. However, in order to emulate
    /// sealed traits you can use this pattern of a public trait in a private inner module. I got
    /// this trick from here: https://github.com/rust-lang/rust/issues/34537
    pub trait VecScopedPrivate {
        type Element;

        fn vec_mut(&mut self) -> &mut Vec<Self::Element>;
    }
}

use private::VecScopedPrivate;
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
        &*self.inner
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
        &*self.0
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

pub struct Assign<'a, V: VecScopedPrivate>(&'a mut V, usize, Option<V::Element>);

impl<'a, V: VecScopedPrivate> Assign<'a, V> {
    pub fn new(vec_scoped: &'a mut V, value: V::Element, idx: usize) -> Self {
        let inner = vec_scoped.vec_mut();
        // i think this is efficient?
        let old = inner.swap_remove(idx);
        let end = inner.len();
        inner.push(value);
        inner.swap(end, idx);
        Self(vec_scoped, idx, Some(old))
    }
}

impl<'a, T, V: std::ops::Deref<Target = [T]> + VecScopedPrivate> std::ops::Deref for Assign<'a, V> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, V: VecScopedPrivate> Drop for Assign<'a, V> {
    fn drop(&mut self) {
        let idx = self.1;
        if let Some(old) = self.2.take() {
            self.vec_mut()[idx] = old;
        }
    }
}

impl<'a, V: VecScopedPrivate> VecScopedPrivate for Assign<'a, V> {
    type Element = V::Element;

    fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
        self.0.vec_mut()
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
fn test_update() {
    let mut a = vec![1];
    {
        assert_eq!([-1], *a.assigned(0, -1));
    }
    assert_eq!([1], *a);

    let mut a = vec![0, 1, 2, 3];
    {
        assert_eq!([0, 1, 5, 3], *a.assigned(2, 5))
    }
    assert_eq!([0, 1, 2, 3], *a);
}

// TODO automatically verify that this warns
#[test]
fn test_must_use() {
    let mut a = vec![1];
    a.pushed(2); // This pushes a value that is then immediately popped, which is useless
    assert_eq!([1], *a);
}
