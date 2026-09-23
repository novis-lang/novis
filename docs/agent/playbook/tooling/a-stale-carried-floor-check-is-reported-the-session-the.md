- **A stale carried floor check is reported the session the floor gate opens, and reads exactly like a
  regression this session caused.** `tools/loop.py`'s `FLOOR_GATE_EVERY` holds the floor on most
  iterations — `.loop/log.md`'s `goal cost` says `(floor gate shut)` over ten checks where an open one
  runs a hundred and eighty — so the earliest-stage red is only the earliest of what ran. Read the
  previous entries' `goal cost` lines before treating an earlier-stage failure as something the last
  commit did. [until: gone tools/loop.py:FLOOR_GATE_EVERY]
