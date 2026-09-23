- **A top-level element of an `array<mixed>` reads fine as a `mixed` argument, which is how a sweep
  over heterogeneous rows is written.** The refusal to index into an `array<mixed>`'s *elements* is
  about the second level only: `$lefts[$i as string]` handed straight to a `mixed` parameter lowers.
  So a table whose rows are an `int`, a `float`, an array and an object is parallel arrays
  (`array<string> $labels`, `array<mixed> $lefts`, `array<mixed> $rights`) plus a counter and a
  `public static function` that asks the members about one row. [until: reviewed 2026-09-06]
