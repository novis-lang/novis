- **A `cargo-named` check runs its test *by name*, so work landed under a different test name leaves
  the check reporting "did not run" over finished code.** The name is the contract the goal was
  authored with, and a check edit has to be made twice — in `docs/agent/loop-goal.toml` and in the
  goal's own copy under `docs/agent/goals/`. Read the stage's `tests = [...]` before naming a test,
  and rename the test to the check when they have already drifted.
  [until: reviewed 2026-09-17]
