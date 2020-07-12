//! Here's a simple example of pushing an element onto a Vec, and having the change automatically
//! undone:
//!
//! ```
//! use scoped_ops::VecScoped;
//!
//! let mut a = vec![1];
//! {
//!     let mut b = a.pushed(2);
//!     assert_eq!(&[1, 2], b.vec_mut().as_slice());
//! }  // b drops, and undoes its change
//!
//! assert_eq!(&[1], a.vec_mut().as_slice());
//! ```

/// This is used to prevent end users from mutating the data structure when it's wrapped in a scoped
/// op. I also tried using a private trait, but it would need to be exposed in a public interface.
struct PrivacyMarker;

/// This trait is a bit like `Iterator` but for scoped operations. Every scoped operation struct
/// (e.g. `Push`) as well the data structure (e.g. `Vec`) must implement this trait.
pub trait VecScoped: Sized {
    type Element;

    fn vec_mut(&mut self, _privacy_marker: PrivacyMarker) -> &mut Vec<Self::Element>;

    // TODO add a way to turn this into a &Vec. Could Deref do the trick?

    fn pushed(&mut self, value: Self::Element) -> Push<Self> {
        self.vec_mut(PrivacyMarker).push(value);
        Push(self)
    }
}

impl<T> VecScoped for Vec<T> {
    type Element = T;

    fn vec_mut(&mut self, _privacy_marker: PrivacyMarker) -> &mut Vec<Self::Element> {
        self
    }
}

// TODO would users ever want to access the element that was popped?
/// This represents that a single element has been pushed onto a Vec.
#[must_use]
pub struct Push<'a, V: VecScoped>(&'a mut V);

impl<'a, V: VecScoped> Drop for Push<'a, V> {
    fn drop(&mut self) {
        let _did_pop = self.0.vec_mut(PrivacyMarker).pop().is_some();
        debug_assert!(_did_pop, "Someone has illicitly popped an element!");
    }
}

impl<'a, V: VecScoped> VecScoped for Push<'a, V> {
    type Element = V::Element;

    fn vec_mut(&mut self, _privacy_marker: PrivacyMarker) -> &mut Vec<Self::Element> {
        self.0.vec_mut(_)
    }
}

#[test]
fn test_scoped_vec() {
    let mut a = vec![1];
    {
        let mut b = a.pushed(2);
        {
            assert_eq!(&[1, 2, 3], b.pushed(3).vec_mut().as_slice());
        }
        assert_eq!(&[1, 2, -3], b.pushed(-3).vec_mut().as_slice());
    }
    assert_eq!(&[1, -2], a.pushed(-2).vec_mut().as_slice());
    assert_eq!(&[1], a.vec_mut().as_slice());
}

// TODO automatically verify that this warns
#[test]
fn test_must_use() {
    let mut a = vec![1];
    a.pushed(2); // This pushes a value that is then immediately popped, which is useless
    assert_eq!(&[1], a.vec_mut().as_slice());
}
