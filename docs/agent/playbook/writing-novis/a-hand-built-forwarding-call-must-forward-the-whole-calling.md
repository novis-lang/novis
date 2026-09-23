- **A hand-built forwarding call must forward the whole calling convention, and the two parameter
  shapes that do not survive abort inside `nvs-runtime` rather than reporting.**
  `rule:types/callable-is-a-closure`'s `(...)` thunk passes its own parameters straight through,
  which is wrong for exactly two: an `inout $x` wants an address where the thunk has an `int`, and a
  variadic tail wants the collected array where it has the first element — the variadic one dies as
  `misaligned pointer dereference` at `nvs-runtime/src/array.rs` with nothing pointing back at the
  callable. Write one case per *declaration* shape the callee can have, not per call site, reading
  `nvs_types::signatures::MethodSig`'s field list for what those shapes are.
  [until: reviewed 2026-09-06]
