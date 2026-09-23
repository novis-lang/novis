- **A `.nvst` case's bindings are script-scoped, `foreach` and `catch` bindings included, so a
  second sweep cannot reuse the first's names at another type.** `foreach ($whole as int $n)` after
  `foreach ($reals as float $n)` is `E0406: … is already declared`, pointing at a line far above;
  the *same* type is fine. Name each binding for what it holds (`$i` for the integer rows), or hoist
  the body into a `public static function` that owns the names. [until: reviewed 2026-09-06]
