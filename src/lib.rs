//! This repo is a 💥 FAILED 💥 ergonomics experiment on a way to temporarily and reversibly modify
//! data structures in Rust.
//!
//! ## The Concept
//!
//! The idea is that it lets you hand a data structure off to another piece of code saying "okay,
//! you can modify this data while you're using it, but you have to put it back to the way you found
//! it." Here's a simple example of letting a user function change a Vec, and having the change
//! automatically undone:
//!
//! ```
//! use scoped_ops::borrowed::{VecScoped, Noop};
//!
//! let user_fn = |a: &mut Noop<Vec<i32>>| {
//!     // these operations modify the Vec in place
//!     let mut b = a.pushed(4);
//!     let c = b.assigned(1, -2);
//!
//!     // the user can now view the changed version of the Vec
//!     assert_eq!([1, -2, 3, 4], *c);
//! };  // c and b drop, and undo their changes
//!
//! let mut a = vec![1, 2, 3];
//! user_fn(&mut a.nooped());
//! assert_eq!([1, 2, 3], *a);
//! ```
//!
//! ## The Approach
//!
//! The approach here gives each mutating operation a scope, reverting the operation when it goes
//! out of scope. So once the operation goes out of scope, the state of the data structure will be
//! returned to what it was before the scoped operation was applied (except for maybe like the
//! capacity of a `Vec` could be different, or something like that). Because each operation is
//! reverted when it goes out of scope, operations can be nested without complication. Conceptually
//! this is like a weaker version of a partially persistent data structure. Unlike in a partially
//! persistent data structure, you can't actually "see" any past state; you have to apply undo
//! operations until you get back to that state.
//!
//! ## The Promise
//!
//! I thought this would be a kind of promising idea:
//!
//! - Uses normal Rust data structures, so can be integrated into existing code
//! - Hopefully zero-cost abstraction, although inspection would be needed to verify this
//! - No dependencies, could work without `std`
//!
//! ## The Reality
//!
//! Unfortunately, this didn't end up being as cool as I envisioned. Why did it fail?
//!
//! - I can't find a real-world use case for this!
//! - Using generics is "viral:" any code that uses this will also need to be generic. This makes
//!   something like looping or recursion a lot harder. You'll also end up with complex nested types
//!   like when using futures or iterators.
//! - Using mutable references, users will need to create too many `let` bindings.
//! - These reversions could in many cases just be coded by hand instead.
//!
//! ## The Alternatives
//!
//! Instead of using this approach, I can think of a few possible alternatives for temporarily and
//! reversibly modifying data:
//!
//! - Ask the user to undo their own changes to the data structure
//! - Just clone the data, then the user can can modify the clone however they want
//! - Apply adapters around the original data structure at read time. For example, you could have a
//!   `VecPushed` that has most of the same methods as `Vec`, but acts as if it has an element
//!   pushed onto the end. For example, if the underlying `Vec` has length 2, the `VecPushed` would
//!   say that its length is 3. Possible disadvantages:
//!   - A lot of code to implement
//!   - Applying a lot of modifications, or certain kinds of modifications, might hurt performance
//!   - You couldn't get a slice, because the data doesn't actually exist in memory
//!
//! ## The Conclusions
//!
//! What I've learned:
//!
//! - Using generics for zero-cost abstractions introduces a usability penalty, because any user
//!   code needs to use the generics. For example, this makes looping pretty annoying.
//! - Using references can be annoying because it limits you to structuring your code so that the
//!   lifetimes work out. In the doctest, I need to create additional `let` bindings that I'd rather
//!   omit.
//! - It's kind of hard to search for "where would this pattern be useful?" I don't know of a good
//!   way to search the entire Rust ecosystem for "unnecessarily cloning a `Vec`" for example.
//! - It's harder to find a problem for a solution than to find a solution to a problem.
//! - Even a failed experiment helped me learn a lot about structuring Rust code. For example, I
//!   learned about:
//!   - It's hard to have both `Drop` and a method that takes `self`
//!   - When to use associated types vs. generic types
//!   - How to emulate a [sealed trait](https://rust-lang.github.io/api-guidelines/future-proofing.html)
//!   - `&mut T` is not [`UnwindSafe`](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html),
//!     meaning I can't test panics with this crate (without learning more about unwind safety).
//!
//! # The Future
//!
//! A few possible interesting angles on future exploration:
//!
//! - Check whether this is actually, as I hoped, a zero-cost abstraction
//! - Figure out if this would actually be useful for anything 😂
//! - Explore support for "commit vs. revert"
//! - Add more operations to `Vec`
//! - Add support for other data structures
//! - Explore a reference-counted variant
//!
//! Thanks to mjhoy for contributing the `Assign` operation, and thanks to everyone on
//! [the URLO thread](https://users.rust-lang.org/t/pattern-for-nested-mutable-references/45651) who
//! suggested great solutions for nesting mutable types!

pub mod borrowed {
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
        /// Temporarily assign an element at `idx` of the `Vec`.
        /// Panics if `idx` is out of bounds.
        fn assigned(&mut self, idx: usize, value: T) -> Assign<Self>
        where
            Self: Sized,
        {
            Assign::new(self, value, idx)
        }

        /// This can be used to turn a `Vec` into a `VecScoped`
        fn nooped(&mut self) -> Noop<Self>
        where
            Self: Sized,
        {
            Noop::new(self)
        }

        /// Temporarily pop the last element from the end of the `Vec`
        fn popped(&mut self) -> Pop<Self>
        where
            Self: Sized,
        {
            Pop::new(self)
        }

        /// Temporarily push an element onto the end of the `Vec`
        fn pushed(&mut self, value: T) -> Push<Self>
        where
            Self: Sized,
        {
            Push::new(self, value)
        }
    }

    impl<T> VecScopedPrivate for Vec<T> {
        type Element = T;

        fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
            self
        }
    }

    impl<T> VecScoped<T> for Vec<T> {}

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

    /// See `crate::borrowed::VecScoped::noop`
    #[must_use]
    pub struct Noop<'a, V: VecScopedPrivate>(&'a mut V);

    impl<'a, V: VecScopedPrivate> Noop<'a, V> {
        pub fn new(vec_scoped: &'a mut V) -> Self {
            Self(vec_scoped)
        }
    }

    impl<'a, T, V: Deref<Target = [T]> + VecScopedPrivate> Deref for Noop<'a, V> {
        type Target = [T];

        fn deref(&self) -> &Self::Target {
            self.0
        }
    }

    impl<'a, V: VecScopedPrivate> VecScopedPrivate for Noop<'a, V> {
        type Element = V::Element;

        fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
            self.0.vec_mut()
        }
    }

    impl<'a, T, V: VecScopedPrivate<Element = T>> VecScoped<T> for Noop<'a, V> {}

    /// See `crate::borrowed::VecScoped::pop`
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

    /// See `crate::borrowed::VecScoped::push`
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
    fn test_noop() {
        let mut a = vec![1, 2, 3];
        {
            assert_eq!([1, 2, 3], *a.nooped());
        }
        assert_eq!([1, 2, 3], *a);
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

    // I don't think this can work b/c the type is different each iteration of the loop. There's a
    // similar issue with recursion. With a complicated enough system of generics this could be used but
    // overall it's probably not worth the trouble.
    //
    // #[test]
    // fn test_loop() {
    //     let mut a: Box<dyn VecScoped<i32>> = Box::new(vec![]);
    //     for i in 0..3 {
    //         a = a.pushed(i);
    //     }
    // }
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

    /// See `crate::owned::VecScoped::pop`
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
