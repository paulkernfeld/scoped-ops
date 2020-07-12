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
pub trait VecScoped: Sized {
    type Element;

    // TODO this should only be accessible from within the trait, we don't want to give users access
    // to modify the vec because they might break an invariant
    fn vec_mut(&mut self) -> &mut Vec<Self::Element>;

    // TODO add a way to turn this into a &Vec. Could Deref do the trick?

    fn pushed(&mut self, value: Self::Element) -> Push<Self> {
        self.vec_mut().push(value);
        Push(self)
    }
}

impl<T> VecScoped for Vec<T> {
    type Element = T;

    fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
        self
    }
}

/// TODO this should probably be must_use
pub struct Push<'a, V: VecScoped>(&'a mut V);

impl<'a, V: VecScoped> Drop for Push<'a, V> {
    fn drop(&mut self) {
        // TODO this doesn't work in release mode
        debug_assert!(
            self.0.vec_mut().pop().is_some(),
            "Someone has illicitly popped an element!"
        );
    }
}

impl<'a, V: VecScoped> VecScoped for Push<'a, V> {
    type Element = V::Element;

    fn vec_mut(&mut self) -> &mut Vec<Self::Element> {
        self.0.vec_mut()
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
