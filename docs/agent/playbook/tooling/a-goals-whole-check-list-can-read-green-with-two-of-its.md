- **A goal's whole check list can read green with two of its stages unbuilt, because a `command`
  check's `want` names the suite's *label* and not a case in it.** `[…, "surfaces:", "0 failing"]` is
  satisfied by whichever cases that directory already holds, so goal `editor-surfaces` would have
  closed with its Test Explorer never written. Read an all-green ledger against the goal prose's
  stages and the `stage =` strings in `loop-goal.toml` — a stage with no check is the hole — and name
  cases in `want` the way `cargo-named` names tests. [until: reviewed 2026-09-11]
