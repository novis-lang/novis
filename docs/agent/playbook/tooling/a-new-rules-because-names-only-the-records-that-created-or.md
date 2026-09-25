- **A new rule's `because` names only the records that created or amended it, and a "depends on"
  record listed there fails `bun nv records --check` against a file nobody may edit.** The relation is
  bidirectional, so every id in a `because` obliges that record's own `changes:` block to name the
  rule back, and a frozen record never acquires one — listing `0051` beside `0179` reported
  `0051.md:3 ... its changes: does not name the rule`. Put the ancestry in the new record's
  `Depends on:` bullet and leave `because` at the records that wrote the rule.
  [until: gone tools/nv/cmd/records.ts:because]
