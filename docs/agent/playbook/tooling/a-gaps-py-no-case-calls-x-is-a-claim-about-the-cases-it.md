- **A `bun nv gaps` "no case calls X" is a claim about the cases it *attributed* to that class, and
  attribution is not coverage.** `coverage`'s attribution follows produced instance types
  (`tools/nv/cmd/gaps.ts`'s `producers`), so a value reached through a factory on another class is
  counted, but a member exercised only through a closure or a helper class the walk cannot follow
  still reads as uncalled — the number is a floor, never a count. `bun nv gaps --member <name>`
  prints the cases that already ask about one: one call, before writing a case the tree already has.
  [until: gone tools/nv/cmd/gaps.ts:producers]
