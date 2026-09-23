- **A module doc section you *write* re-breaks a floor check an earlier goal closed tree-wide, because
  `tools/owners.py` reads a heading's wording and never what the section says.**
  `# Reaching a core that has not started yet` described a landed handle and owed nothing, but `OWED`
  matches `not … yet` in a title, so `sections outside Known gaps: 0` was red for a whole goal. Title a
  new `//!` section for what the module does, and run `python tools/owners.py | grep "sections
  outside"` in any session that adds one. [until: gone tools/owners.py:sections outside Known gaps]
