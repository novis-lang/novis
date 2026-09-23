- **A `cargo-named` check counts an `ignored` test as one that ran, so a guard copied from
  `perf_guards.rs` reports green while measuring nothing.** `tools/loop.py:2350` asks only whether
  the test's name is in the output and `test <name> ... ignored` names it, while `perf_guards.rs`
  gates each guard behind `#[cfg_attr(debug_assertions, ignore)]` and no check's `args` carries
  `--release`. Put the ceiling behind `#[cfg(debug_assertions)]` instead, as
  `crates/nvs-lsp/tests/latency.rs` does, so the guard runs in the build the gate builds.
  [until: gone tools/loop.py:name not in both]
