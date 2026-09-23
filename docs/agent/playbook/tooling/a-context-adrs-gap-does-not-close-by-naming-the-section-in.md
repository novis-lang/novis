- **A [context] adrs gap does not close by naming the section in the handoff item.** `orient.py`
  slices the `[context] adrs` and `rules` lists and nothing else — it never reads an item's own
  `rule:` tokens — so a section asked for in the handoff is re-sliced by hand session after session
  while the list stays unchanged. Edit `docs/agent/loop-goal.toml` in the session that discovers the
  gap; `tools/loop.py` reloads it every iteration. [until: gone tools/orient.py:adrs]
