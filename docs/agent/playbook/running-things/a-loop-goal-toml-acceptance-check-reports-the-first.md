- **A `loop-goal.toml` acceptance check reports the *first* diagnostic, not the tree's whole
  distance from passing.** A check that has said `E0703 — 'spawn script' is not compiled yet` for
  sessions reads as one construct away when `nvs check` on the same file reports two more, one of
  them a construct nowhere in `nvs-syntax`'s AST. One `./target/debug/nvs.exe check <file>` of a
  failing `exact` check's own fixture, before planning the group that closes it, is the difference
  between a group and a milestone. [until: reviewed 2026-09-06]
