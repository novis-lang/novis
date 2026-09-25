- **A test binary outside `nvs-runtime` measures allocations through `nvs_runtime::budget`, and may
  not install a `#[global_allocator]` of its own.** `budget::live_bytes`, `::allocated_bytes` and
  `::allocations` are `pub` and maintained in every profile, and `nvs-runtime` registers
  `budget::Accounting` in every `not(test)` build, so a second global allocator in a test file is
  `error: the #[global_allocator] in this crate conflicts with global allocator in: nvs_runtime`.
  `crates/nvs-stdlib/tests/allocation_policy.rs` reads the shared counters.
  [until: gone crates/nvs-stdlib/tests/allocation_policy.rs:nvs_runtime::budget]
