- **`orient.py` prints less than `brief.py` on purpose, and the gap is a bug in the goal, not in the
  tool.** If it did not print a module, a rule section, a convention shape or a playbook bullet you
  turned out to need, do not re-run `brief.py` for everything. Fetch the one thing, then edit that
  `[context]` field in `loop-goal.toml` yourself — it reloads every session and widening it breaks
  nothing, while a gap only written into the handoff is one every later session pays too. `modules`
  is the field you may leave: `tools/context-sync.py` sweeps it from your commits between sessions.
  [until: gone tools/context-sync.py:MAX_ADDED]
