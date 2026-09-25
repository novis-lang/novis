- **A `@symbol` target names the member alone, so `@Registry::of` answers *no definition or
  mention* — which reads like the symbol has been deleted.** `bun nv peek` matches a Rust definition
  by its own name, and a method's name does not carry its type; the qualified spelling is the one a
  `Core` member and a `.nvst` case take. Ask for `@of`, or `--locate of`, and read the `impl` the hit
  lands in. [until: gone tools/nv/cmd/peek.ts:path:@name]
