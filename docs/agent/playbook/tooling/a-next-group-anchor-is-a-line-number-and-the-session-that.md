- **A `## Next group` anchor is a line number, and the session that wrote it usually went on editing
  that same file.** The windows `orient.py` inlines for a stale anchor are correct-looking code from
  the wrong place, with nothing in the pack to say so. Trust the symbol name in the item, not the
  code beside it — `python tools/peek.py --locate <sym> ...` re-derives every anchor in one call —
  and resolve the next group's anchors after the last commit, not before it.
  [until: reviewed 2026-09-06]
