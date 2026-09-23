- **A `peek.py` `re:` target over `docs/agent/loop-goal.toml` sweeps every stage of a file thousands
  of lines long, and the word you searched for is prose in most of them.** A bare word matches
  `[context]`, unrelated stage comments and a dozen `tests = [...]` lists, costing many times the
  two blocks wanted. Anchor on the syntax around what you want — `re:stage = "3` for a stage's own
  blocks, `re:pub fn <name>` for a definition — or read the window once you know the line.
  [until: reviewed 2026-09-06]
