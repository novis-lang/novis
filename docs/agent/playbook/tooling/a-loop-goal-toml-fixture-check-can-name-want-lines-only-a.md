- **A `loop-goal.toml` fixture check can name `want` lines only a *leg argument* can produce, and the
  missing half is the check's `args`, not the program.** A check whose comment says it "runs under
  `nvs run --request`" while passing no `--request` leaves the fixture printing a refusal forever,
  which reads exactly like unwritten work. `bun nv loop --list` prints each fixture's whole
  argv: read that before believing a `want` cannot be produced, and fix
  `docs/agent/goals/<goal>.toml` too. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
