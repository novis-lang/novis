- **A goal that overturns a behaviour leaves the carried floor checks asserting the old one, and
  nothing goes red until `session.py --wrap` gates a `DONE`.** A `cargo-named` check matches a test
  by name, so one deleted by the work that made it false reads as `did not run` rather than as a
  failure, on a floor that runs one session in ten. When a slice deletes or renames a test, grep its
  name in
  `docs/agent/loop-goal.toml` *and* `docs/agent/goals/<n>-<slug>.toml` in that same slice and
  re-point the check at what now holds the truth. [until: reviewed 2026-09-15]
