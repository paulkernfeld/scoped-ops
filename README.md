This repo is an ergonomics experiment on a way to temporarily and reversibly modify data structures in Rust. The idea is
that it lets you hand a data structure off to another piece of code saying "okay, you can modify this data while you're
using it, but you have to put it back to the way you found it." The approach here gives each mutating operation a scope,
reverting the operation when it goes out of scope. So once the operation goes out of scope, the state of the data
structure will be returned to what it was before the scoped operation was applied (except for maybe like the capacity of
a `Vec` could be different, or something like that). Because each operation is reverted when it goes out of scope,
operations can be nested without complication. Conceptually this is like a weaker version of a partially persistent data
structure.

Advantages:

- Uses normal Rust data structures, so can be integrated into existing code
- Hopefully zero-cost abstraction, although inspection would be needed to verify
- No dependencies, could work without `std`

Disadvantages:

- These reversions could in many cases just be coded by hand
- Possibly slow compilation
- The generics are kind of a beast; you'll end up with complex nested types like when using futures or iterators
- The syntax is a bit long and indent-y for my tastes

Todo:
- Add a popping type
