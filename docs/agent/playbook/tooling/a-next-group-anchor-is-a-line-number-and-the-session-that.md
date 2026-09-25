- **A `## Next group` anchor is a line number, and the session that wrote it usually went on editing
  that same file.** The windows `bun nv orient` inlines for a stale anchor are correct-looking code from
  the wrong place, with nothing in the pack to say so. Trust the symbol name in the item, not the
  code beside it — `bun nv peek --locate <sym> ...` re-derives every anchor in one call —
  and resolve the next group's anchors after the last commit, not before it.
  [until: gone tools/nv/cmd/orient.ts:## Next group]
