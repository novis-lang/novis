- **`bun nv api-rename` lists `->old(` but not a generic call `->old<T>(`, and a regex over `{ … old: }`
  also rewrites shape types.** Stage 5 of `clear-member-names` left every `->streamAs<Row>(` call
  standing until a subagent found them, and its `nanos:` sweep renamed the extension record field in
  `array<{ …, nanos: int }>`, which the extension's WIT type fixes. Grep for `->old<` beside `->old(`,
  and restrict an option-field rewrite to bags after a call (`({`), never a type. See
  `tools/nv/cmd/api-rename.ts`.
