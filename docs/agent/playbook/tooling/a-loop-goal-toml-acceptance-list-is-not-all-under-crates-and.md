- **A `loop-goal.toml` acceptance list is not all under `crates/` and `tests/` —
  `benches/abi-probe/` hosts a whole leg of it.** `perf_guards.rs` and `invariants.rs` under
  `benches/abi-probe/tests/` are the ABI and cost-class guards, a workspace member the goal names
  like any other, and the only tests outside those two roots. Grep `fn <name>` from the repository
  root, or you will file a slice to write a test that already exists.
  [until: gone benches/abi-probe/tests/perf_guards.rs]
