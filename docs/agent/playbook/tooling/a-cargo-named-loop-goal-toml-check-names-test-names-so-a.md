- **A `cargo-named` `loop-goal.toml` check names *test names*, so a test that pins the same
  behaviour under a different name does not close it.** The driver reports `did not run` every
  iteration while the assertion sits green on disk under its own spelling. `grep -rn` the drafted
  name across `crates/` before writing anything, then **rename** the existing test rather than
  adding a second copy — a `///` in a third file may already link the drafted name, and two tests
  asserting one thing is how the next session loses an hour. [until: reviewed 2026-09-06]
