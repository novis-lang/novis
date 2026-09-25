- **A `[context]` gap is closed by adding the path to `modules`, and there is no `files` field.**
  `bun nv orient` resolves every leftover `modules` pattern against the tracked files, so a goal may
  name a fixture, a tool or a `.nvst` case there, while a key the loader does not know is silently
  nothing at all. Add the path with `bun nv goal context --add <path>`, or put it in a
  `[context.stage.N] modules` overlay when only one stage reads it.
  [until: gone tools/nv/cmd/orient.ts:any tracked file resolves]
