- **A loop-goal.toml check can name something that is not a test at all, and the tell is the *check
  count* rather than the message.** A `cargo-named` check listing an `examples/*.nvs` path beside
  real test names fails forever, since a program leg cannot appear in `cargo test`'s output, and the
  driver stops there — `over 4 check(s)` in `.loop/log.md` where a healthy iteration reads over a
  hundred. Check that a name *could* be a `#[test]` before writing one, and fix
  `docs/agent/goals/<goal>.toml` alongside the live copy, or the next `goal-switch.py` carries it
  back. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
