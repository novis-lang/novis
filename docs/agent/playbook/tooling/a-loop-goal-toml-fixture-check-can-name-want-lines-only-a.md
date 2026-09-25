- **A goal record's fixture check can name `want` lines only a *leg argument* can produce, and the
  missing half is the check's `args`, not the program.** A check the goal's prose says "runs under
  `nvs run --request`" while its `args` pass no `--request` leaves the fixture printing a refusal
  forever, which reads exactly like unwritten work. `bun nv loop --list` prints each fixture's whole
  argv: read that before believing a `want` cannot be produced, and fix the `args` in
  `data/goals/<slug>.json`. [until: gone tools/nv/cmd/orient.ts:const TRIAGE]
