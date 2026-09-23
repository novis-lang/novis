- **A `did not run` naming an *early* stage is the acceptance frontier having moved backwards past
  the handoff's next group.** The driver walks the stages in order and stops at the first failure,
  so closing a later check can expose a test an earlier stage named that never existed, and every
  session since had been reading a report that happened to name something later. The tell is the
  check count in `.loop/log.md` rising as earlier stages pass; grep the named test across
  `crates/**/*.rs` and read the check's `tests` list in `loop-goal.toml` before taking the handoff's
  group at all. [until: reviewed 2026-09-06]
