- **A `python - <<'PY'` regex over a whole source file is AGENTS.md rule 1's breach with a different
  spelling, and it fails the same way a `sed -i` does: silently, far from where you were looking.**
  A non-greedy pattern has no idea what a Rust item is and can rewrite a function signature hundreds
  of lines from the call sites it was aimed at. Use `Edit`'s `replace_all` when the string is
  literal and unique and `tools/splice.py --patch` when it is not; a script that must exist uses
  exact literals with an asserted count, never a regex with `.` in it. [until: reviewed 2026-09-06]
