- **A `[context]` gap is closed by adding the path to `modules`, and there is no `files` field.**
  `tools/orient.py`'s `named_files` resolves every leftover `modules` pattern against `git ls-files`,
  so a goal may name a fixture, a tool or a `.nvst` case there, while a key the loader does not know
  is silently nothing at all. Put the path in `[context] modules`, or in a `[context.stage.N]
  modules` overlay when only one stage reads it, with the one-line comment the other entries carry.
  [until: gone tools/orient.py:named_files]
