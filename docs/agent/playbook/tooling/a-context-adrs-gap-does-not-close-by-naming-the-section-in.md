- **A [context] adrs gap does not close by naming the section in the handoff item.** `bun nv orient`
  slices the goal's `context.adrs` and `context.rules` lists and nothing else — it never reads an
  item's own `rule:` tokens — so a section asked for in the handoff is re-sliced by hand session
  after session while the list stays unchanged. Add it to `context` in the goal's record,
  `data/goals/<slug>.json`, in the session that discovers the gap; `bun nv loop` reloads it every
  turn. [until: gone tools/nv/cmd/orient.ts:adrs]
