- **Measure compiled code's allocations as a difference between two run lengths or two arities,
  never as an absolute zero.** A run allocates its array, locals and output buffer once, so "400
  passes allocated what 4 did" holds where "allocated nothing" cannot. Pair it with a control arm
  that *does* allocate per access. `call_callable` builds up to `INLINE_SLOTS` argument slots on
  the stack and only a longer call spills to the heap, so a callback with more arguments than that
  allocates once per call. See `crates/nvs-runtime/src/callable.rs`.
  `until: gone crates/nvs-runtime/src/callable.rs:INLINE_SLOTS`
