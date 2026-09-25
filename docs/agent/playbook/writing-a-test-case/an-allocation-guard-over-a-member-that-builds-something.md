- **An allocation guard over a member that *builds* something measures the result's own storage
  first.** A `counting_alloc::allocated_bytes` delta over one `map`-shaped walk into a fresh
  `NvsArray` counts the output's own `Vec` doubling, which the guard cannot tell from the allocation
  it exists to catch. Walk twice and measure the *second* pass, where every write lands at a
  position that already exists; `a_callback_that_does_not_want_a_key_synthesizes_none` is the shape.
  [until: gone crates/nvs-runtime/src/array.rs:a_callback_that_does_not_want_a_key_synthesizes_none]
