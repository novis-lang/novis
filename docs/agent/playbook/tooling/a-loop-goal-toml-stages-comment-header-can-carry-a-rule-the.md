- **A `loop-goal.toml` stage's *comment header* can carry a rule the orientation pack never
  prints.** `bun nv orient` prints the item, the standing decisions and the rule sections, not the
  TOML's own comments — and a `[[check]]` block's comment can forbid the obvious fix (*"Do not
  rename one to something already green"*). When an acceptance failure names a test that "did not
  run", `sed -n` the twenty lines around its `[[check]]` block before deciding what the failure
  means; that block, comment included, is the specification. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
