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
