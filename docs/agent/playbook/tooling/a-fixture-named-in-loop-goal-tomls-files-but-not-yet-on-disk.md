- **A fixture named in `loop-goal.toml`'s `files` but not yet on disk aborts the *whole* acceptance
  check, before the first build.** `begin` in `tools/loop.py` walks `files` and returns on the first
  missing one, so the ledger reads `0s over 1 check(s)` and not one floor check runs — a goal that
  adds fixtures for unwritten features has no regression coverage at all until every one of them
  exists. Write the fixture the moment the goal names it, red or not: the toml's own header says a
  fixture's source is not frozen precisely because its author could not compile it.
  [until: gone tools/loop.py:is missing -- the acceptance fixtures are fixed]
