- **Measure compiled code's allocations as a difference between two run lengths or two arities,
  never as an absolute zero.** A run allocates its array, locals and output buffer once, and
  `call_closure` its retained argument slice once per call, so "400 passes allocated what 4 did"
  holds where "allocated nothing" cannot. Pair it with a control arm that *does* allocate per
  access; only with no callback (`sort($list)`) is an absolute bound right.
  [until: gone crates/nvs-runtime/src/closure.rs:call_closure]
