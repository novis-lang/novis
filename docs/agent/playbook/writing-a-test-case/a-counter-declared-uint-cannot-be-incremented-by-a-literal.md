- **A counter declared `uint` cannot be incremented by a literal.** `uint $n = 0; $n = $n + 1;` is
  `E0407: int and uint have no representable common type in arithmetic`, because the literal is an
  `int`, followed by an `E0401` reporting the result as `mixed`. The spelling that compiles is `$n =
  $n + (1 as uint);` — parenthesised, since `as` binds looser than `+` — and it is worth it whenever
  the total is compared against `Core\Arr::count`, which answers `uint`.
  [until: reviewed 2026-09-06]
