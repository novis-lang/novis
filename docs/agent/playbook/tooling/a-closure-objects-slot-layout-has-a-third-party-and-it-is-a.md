- **A closure object's slot layout has a third party, and it is a test in `nvs-stdlib`.**
  `closure_of` in `crates/nvs-stdlib/tests/allocation_policy.rs` hand-builds a closure with the
  reserved slots and a Rust `invoke`, so adding a reserved slot in `nvs-ir` breaks it as `field slot
  N is out of range for a class with N slots` from `nvs_runtime::object`, three crates from the
  edit. `grep -rn CLOSURE_ARITY_SLOT --include=*.rs crates/` finds every builder in one call; do
  that before moving the layout, not after. [until: reviewed 2026-09-06]
