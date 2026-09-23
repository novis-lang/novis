- **A module's gap numbers shift the moment a bullet above one is closed, so a goal list or a handoff
  item naming `gap 2` can point at a different bullet than it did when it was written.**
  `tools/owners.py` numbers a `# Known gaps` bullet by its position in the block, and closing the one
  above it renumbers every one below without touching a word of the prose that names them elsewhere.
  Match on the gap's *sentence*, not its digit — `python tools/owners.py | grep <file>` prints each
  open bullet's opening line beside the number it currently has. [until: reviewed 2026-12-16]
