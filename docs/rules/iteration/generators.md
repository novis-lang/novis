A function whose body contains `yield` is a **generator**. Its declared return type must be
`Iterator<T>`, and every `yield` operand is checked against that `T`. Calling it runs no user code:
it allocates and returns the state object, which implements `Iterator<T>`, and the body then runs one
segment per `advance()`. Two calls to the same method give two cursors that advance independently.

The body is compiled by an explicit **state-machine transform** — split at each `yield` into
resumption states reached through an N-way switch, with every local live across a `yield` parked in
the state object rather than on a stack. A generator is therefore an ordinary object: ten thousand
live ones cost ten thousand small objects, not ten thousand stacks, and suspension stays a property
of I/O rather than of ordinary control flow.

A `finally` the generator is suspended inside still runs when the consumer abandons it, and
definite assignment reasons across resumption edges exactly as it does across a linear body.

The lowering does not use the coroutine substrate, because a language feature must behave the same
on every compile target, and a runtime facility such as stack switching is allowed to differ between
them. A target without it would otherwise lose generators or need a second lowering, and the second
lowering is this one.

**Memory:** each live generator holds one state object with a field for every local live across a
`yield`, for as long as the generator object lives.

**Declined:**
- **Generators on the coroutine substrate.** One suspension mechanism instead of two, and `yield`
  from any call depth, but a stack per live generator and suspension no longer a property of I/O
  alone, to buy yielding from a helper, which is rare and always rewritable as a loop
  (`rule:iteration/yield-lexical-confinement`).
- **No generators, with `Iterator` implemented by hand.** The smallest surface, but every lazy
  pipeline in a program becomes an explicit state class holding exactly the state the compiler would
  have generated correctly. The library's native iterators cover the common cases and cannot cover a
  program's own transformation of one.
