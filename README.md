# scoped-ops

This repo is an ergonomics experiment on a way to temporarily and reversibly modify data structures in Rust. The idea is
that it lets you hand a data structure off to another piece of code saying "okay, you can modify this data while you're
using it, but you have to put it back to the way you found it." The approach here gives each mutating operation a scope,
reverting the operation when it goes out of scope. So once the operation goes out of scope, the state of the data
structure will be returned to what it was before the scoped operation was applied (except for maybe like the capacity of
a `Vec` could be different, or something like that). Because each operation is reverted when it goes out of scope,
operations can be nested without complication. Conceptually this is like a weaker version of a partially persistent data
structure.

Here's a simple example of pushing an element onto a Vec, and having the change automatically
undone:

```rust
use scoped_ops::VecScoped;

let mut a = vec![1];
{
    // these operations modify the Vec in place
    let mut b = a.pushed(2);
    let c = b.assigned(0, 5);
    assert_eq!([5, 2], *c);
}  // c and b drop, and undo their changes

assert_eq!([1], *a);
```

Advantages:

- Uses normal Rust data structures, so can be integrated into existing code
- Hopefully zero-cost abstraction, although inspection would be needed to verify
- No dependencies, could work without `std`

Disadvantages:

- I can't find a real-world use case for this!
- Using generics is "viral:" any code that uses this will also need to be generic, which will
  make looping a lot harder.
- These reversions could in many cases just be coded by hand
- Possibly slow compilation
- The generics are kind of a beast; you'll end up with complex nested types like when using futures or iterators
- The syntax is a bit long and indent-y for my tastes

I can think of a few possible alternatives for temporarily and reversibly modifying data:

- Just clone the data, then you can modify the clone however you want
- Trust the user to take care of it
- Apply adapters around the original data structure at read time. For example, you could have a `VecPushed` that has
  most of the same methods as `Vec`, but acts as if it has an element pushed onto the end. For example, if the
  underlying `Vec` has length 2, the `VecPushed` would say that its length is 3. Possible disadvantages:
  - A lot of code to implement
  - Applying a lot of modifications, or certain kinds of modifications, might hurt performance
  - You couldn't get a slice, because the data doesn't actually exist in memory

To do:

- Explore support for "commit vs. revert"?
- Add support for other data structures?
- Explore reference-counted variant?
- Figure out if this would actually be useful for anything 😂?
