- **`check_expr` reports the mismatch itself, so a pass that checks an argument only to *learn* its
  type must call `infer`.** `crates/nvs-types/src/expr/mod.rs:129` compares the result against the
  expectation and reports `E0401` there, which is why `check_generic_args`' first pass hands an
  expectation only to positions that are already final — a mid-pass check against a
  half-substituted type reports a mismatch the last pass would have reported correctly, and the
  message names the unsubstituted variable's `mixed`. Call `infer` where the expectation is there to
  place a literal rather than to judge it. [until: reviewed 2026-09-07]
