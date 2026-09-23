- **A rule's table row can be stale about the tree the way a `loop-goal.toml` comment can; the fix
  is to amend the rule.** `rule:programs/framework-core-half`'s `Core\Cldr` row claimed data
  `nvs_stdlib::cldr` never held, so the row was a member *and* a ~170-language table, a different
  size of slice than it predicts. The same one `grep` per claim applies, but a rule's body always
  states the current rule, so the stale row is a bug to fix in the same session; read the row's
  reason apart from its claim about the tree, since only the claim was wrong.
  [until: reviewed 2026-09-06]
