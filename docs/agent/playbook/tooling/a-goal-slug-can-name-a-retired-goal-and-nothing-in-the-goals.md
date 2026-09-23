- **A goal slug can name a *retired* goal, and nothing in the goals directory says which those are.**
  `chain.py --check` accepts the tag because a retired goal keeps its `N-<slug>.md` and loses only its
  `.toml` and `.handoff.md`, so `owners.py` takes it and then reports it as an owner that went green
  without closing its gap — a finding a later session has to unpick. `ls docs/agent/goals/*.toml` is
  the live set; everything else is walked, which also means its milestone counts as *carried* and its
  leftover gaps are `unowned` rather than that milestone's. [until: reviewed 2026-09-10]
