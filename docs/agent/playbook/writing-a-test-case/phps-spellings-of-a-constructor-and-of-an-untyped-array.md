- **PHP's spellings of a constructor and of an untyped array local do not compile in a `.nvst`
  case.** The constructor is `constructor`, not `__construct` (E0114, then E0409 for every property
  it would have assigned), and an array local written with no type, `$x = [1, 2, 3];`, is E0301.
  Write `public function constructor(...)` and `var $x = [1, 2, 3];` or `array<int> $x = [1, 2, 3];`;
  `var` over a literal whose elements have two types, or over an empty one, is E0414.
  [until: gone crates/nvs-diagnostics/src/lib.rs:E0414]
