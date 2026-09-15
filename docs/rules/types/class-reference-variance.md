`class<Dog>` widens to `class<Animal>` wherever `Dog` widens to `Animal` — a parameter, a return, an
assignment to a wider binding — and **never back**. A narrowing is written like every other narrowing,
`as class<Dog>`, and is checked against the descriptor at run time
(`rule:types/class-reference`).

This is covariance, and it is sound here for the reason it is unsound for a mutable container: a
descriptor has no write side. There is nothing to put into a `class<T>`, so the argument's position is
purely an output and the usual variance trap has nothing to catch. `array<T>` reaches the same answer
by the other road — it has a write side, and copy-on-write value semantics keep a write through the
widened view out of the narrow one's storage (`rule:types/arrays`) — which leaves `property<T>` the
one that widens on neither argument, because its argument bounds a receiver instead
(`rule:types/property-key-variance`).
