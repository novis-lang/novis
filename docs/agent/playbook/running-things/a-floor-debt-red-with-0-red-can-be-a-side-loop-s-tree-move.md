- **A `floor-debt` red with "0 red" in every round can be a side loop's uncommitted edits, not a
  failure.** The run pays at most three rounds, and an edit another loop makes to `tools/nv/` while
  it runs owes its atoms again after the last round. Read `looper status` and `git status` for the
  other loop's files, list the store's debt with `debtOf(new SelectStore().atoms())`, run the named
  test once by name, and pay it with `bun nv affected --run`; the next `floor-debt` pays the rest
  quickly. See `tools/nv/cmd/debt.ts`. `until: gone tools/nv/cmd/debt.ts:const ROUNDS = 3`
