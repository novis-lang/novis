- **`bun nv gaps --differential` matches text, so a member can leave the list without a case of its
  own.** `calledMembers` in `tools/nv/cmd/gaps.ts` is one regex over the whole case text and never
  looks for a `(`, so a comment naming `Core\Path::normalize` inside a `basename` case silences
  `normalize`'s row, and an oracle's fallback call to a sibling member does the same. Name a
  neighbour without its class, re-run `--differential` after writing a case, and when the drop is
  larger than the members you asked about, the extra one still owes a case.
  [until: gone tools/nv/cmd/gaps.ts:calledMembers]
