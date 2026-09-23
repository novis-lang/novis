- **A `loop-goal.toml` check whose comment says a slice is unlanded can be right, and one grep for
  the surface settles it either way.** The sibling triages assume a misfiled check; the other
  outcome is a real gap that several module docs each name as "not landed". One `grep -rn` for the
  surface's own name across the crates answers where the rule is named, where it is implemented, and
  which doc comments go stale when it lands — that list is also the edit list.
  [until: reviewed 2026-09-06]
