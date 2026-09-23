- **`var` takes no type annotation, and writing one costs four diagnostics a line.** `var string $s
  = …` makes the parser expect a name, find `string`, and emit `E0101` twice, `E0102`, a third
  `E0101` and an `E0406` claiming `$` is already declared — none says "a `var` has no type". The
  spellings are `var $s = …` (inferred) and `string $s = …` (declared); look for the second
  `E0101`'s "`var` infers its type from the initializer". [until: reviewed 2026-09-06]
