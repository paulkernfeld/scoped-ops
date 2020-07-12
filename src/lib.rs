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
    pub trait VecScopedPrivate {
        type Element;

        // TODO can we get rid of this and just use Deref?
        fn as_slice(&self) -> &[Self::Element];

        fn vec_mut(&mut self) -> &mut Vec<Self::Element>;
    }
}

use private::VecScopedPrivate;

pub trait VecScoped<T>: Sized + VecScopedPrivate<Element = T> {
    fn pushed(&mut self, value: T) -> Push<Self> {
        self.vec_mut().push(value);
        Push(self)
    }
}

impl<T> VecScopedPrivate for Vec<T> {
    type Element = T;

    fn as_slice(&self) -> &[Self::Element] {
        Vec::as_slice(self)
    }

    fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
        self
    }
}

impl<T> VecScoped<T> for Vec<T> {}

// TODO would users ever want to access the element that was popped?
/// This represents that a single element has been pushed onto a Vec.
#[must_use]
pub struct Push<'a, V: VecScopedPrivate>(&'a mut V);

impl<'a, V: VecScopedPrivate> std::ops::Deref for Push<'a, V> {
    type Target = [V::Element];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
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

    fn as_slice(&self) -> &[Self::Element] {
        self.0.as_slice()
    }

    fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
        self.0.vec_mut()
    }
}

impl<'a, T, V: VecScopedPrivate<Element = T>> VecScoped<T> for Push<'a, V> {}

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

// TODO automatically verify that this warns
#[test]
fn test_must_use() {
    let mut a = vec![1];
    a.pushed(2); // This pushes a value that is then immediately popped, which is useless
    assert_eq!([1], *a);
}
