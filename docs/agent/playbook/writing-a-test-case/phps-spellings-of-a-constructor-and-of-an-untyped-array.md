- **PHP's spellings of a constructor and of an untyped array local do not compile in a `.nvst`
  case.** The constructor is `constructor`, not `__construct` (E0114, then E0409 for every property
  it would have assigned), and `var $x = [1, 2, 3];` is E0414 because an array literal has no target
  type to infer from. Write `array<int> $x = [1, 2, 3];` and `public function constructor(...)`.
  [until: reviewed 2026-09-06]
