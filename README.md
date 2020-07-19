# scoped-ops

This repo is a 💥 FAILED 💥 ergonomics experiment on a way to temporarily and reversibly modify
data structures in Rust.

### The Concept

The idea is that it lets you hand a data structure off to another piece of code saying "okay,
you can modify this data while you're using it, but you have to put it back to the way you found
it." Here's a simple example of letting a user function change a Vec, and having the change
automatically undone:

```rust
use scoped_ops::borrowed::{VecScoped, Noop};

let user_fn = |a: &mut Noop<Vec<i32>>| {
    // these operations modify the Vec in place
    let mut b = a.pushed(4);
    let c = b.assigned(1, -2);

    // the user can now view the changed version of the Vec
    assert_eq!([1, -2, 3, 4], *c);
};  // c and b drop, and undo their changes

let mut a = vec![1, 2, 3];
user_fn(&mut a.nooped());
assert_eq!([1, 2, 3], *a);
```

### The Approach

The approach here gives each mutating operation a scope, reverting the operation when it goes
out of scope. So once the operation goes out of scope, the state of the data structure will be
returned to what it was before the scoped operation was applied (except for maybe like the
capacity of a `Vec` could be different, or something like that). Because each operation is
reverted when it goes out of scope, operations can be nested without complication. Conceptually
this is like a weaker version of a partially persistent data structure. Unlike in a partially
persistent data structure, you can't actually "see" any past state; you have to apply undo
operations until you get back to that state.

### The Promise

I thought this would be a kind of promising idea:

- Uses normal Rust data structures, so can be integrated into existing code
- Hopefully zero-cost abstraction, although inspection would be needed to verify this
- No dependencies, could work without `std`

### The Reality

Unfortunately, this didn't end up being as cool as I envisioned. Why did it fail?

- I can't find a real-world use case for this!
- Using generics is "viral:" any code that uses this will also need to be generic. This makes
  something like looping or recursion a lot harder. You'll also end up with complex nested types
  like when using futures or iterators.
- Using mutable references, users will need to create too many `let` bindings.
- These reversions could in many cases just be coded by hand instead.

### The Alternatives

Instead of using this approach, I can think of a few possible alternatives for temporarily and
reversibly modifying data:

- Ask the user to undo their own changes to the data structure
- Just clone the data, then the user can can modify the clone however they want
- Apply adapters around the original data structure at read time. For example, you could have a
  `VecPushed` that has most of the same methods as `Vec`, but acts as if it has an element
  pushed onto the end. For example, if the underlying `Vec` has length 2, the `VecPushed` would
  say that its length is 3. Possible disadvantages:
  - A lot of code to implement
  - Applying a lot of modifications, or certain kinds of modifications, might hurt performance
  - You couldn't get a slice, because the data doesn't actually exist in memory

### The Conclusions

What I've learned:

- Using generics for zero-cost abstractions introduces a usability penalty, because any user
  code needs to use the generics. For example, this makes looping pretty annoying.
- Using references can be annoying because it limits you to structuring your code so that the
  lifetimes work out. In the doctest, I need to create additional `let` bindings that I'd rather
  omit.
- It's kind of hard to search for "where would this pattern be useful?" I don't know of a good
  way to search the entire Rust ecosystem for "unnecessarily cloning a `Vec`" for example.
- It's harder to find a problem for a solution than to find a solution to a problem.
- Even a failed experiment helped me learn a lot about structuring Rust code. For example, I
  learned about:
  - It's hard to have both `Drop` and a method that takes `self`
  - When to use associated types vs. generic types
  - How to emulate a [sealed trait](https://rust-lang.github.io/api-guidelines/future-proofing.html)
  - `&mut T` is not [`UnwindSafe`](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html),
    meaning I can't test panics with this crate (without learning more about unwind safety).

## The Future

A few possible interesting angles on future exploration:

- Check whether this is actually, as I hoped, a zero-cost abstraction
- Figure out if this would actually be useful for anything 😂
- Explore support for "commit vs. revert"
- Add more operations to `Vec`
- Add support for other data structures
- Explore a reference-counted variant

Thanks to mjhoy for contributing the `Assign` operation, and thanks to everyone on
[the URLO thread](https://users.rust-lang.org/t/pattern-for-nested-mutable-references/45651) who
suggested great solutions for nesting mutable types!
