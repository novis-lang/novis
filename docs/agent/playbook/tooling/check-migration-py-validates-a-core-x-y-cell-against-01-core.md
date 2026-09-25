- **`bun nv migration` validates a `Core\X::y` cell against `01-core-library.md`'s *backticked
  words*, not against the registry.** Its extraction takes a bare backticked word, a `name(` prefix
  or `$var->name`, so a member the spec spells only as `implementing<T>()` or `->readLine` fails
  with "which 01-core-library.md does not" — a spec-side gap that reads as a misspelling. Do not
  widen the regex in `tools/nv/cmd/migration.ts`'s `coreSurface`: write the handle form
  (`$file->readLine`), name the class and leave the member in prose, or give 01 a machine-readable
  spelling. [until: gone tools/nv/cmd/migration.ts:coreSurface]
