- **The live acceptance list and the goal's own `docs/agent/goals/<goal>.toml` drift apart, and a
  plain `diff` will not show you how far.** The live `docs/agent/loop-goal.toml` is written with CRLF
  while the source stays LF, so `diff` reports every line changed and reads as two unrelated files —
  and because `goal-switch.py` carries the *live* file's checks forward, a stale source is invisible
  until something reinstalls it. Use `diff --strip-trailing-cr` before believing either file, and put
  an amendment to a check into both in the same commit. [until: reviewed 2026-09-08]
