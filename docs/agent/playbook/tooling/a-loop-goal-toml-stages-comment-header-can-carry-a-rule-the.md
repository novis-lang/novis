- **A goal record's stage has its prose in the goal's `## Stage` section, which can carry a rule
  the orientation pack never prints.** `bun nv orient` prints the item, the standing decisions and
  the rule sections, not the goal's stage sections — and a stage section can forbid the obvious fix
  (*"Do not rename one to something already green"*). When an acceptance failure names a test that
  "did not run", read that stage's section in `docs/agent/goals/<slug>.md` and the check's line in
  `bun nv loop --list --stage <n>` before deciding what the failure means; the two together are the
  specification. [until: gone tools/nv/cmd/orient.ts:const TRIAGE]
