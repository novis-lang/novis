- **A bench that reaches its subject array through a nested array index charges the member two
  allocations per op.** `benches/members/core/Arr/first.nvs` chained its rounds through
  `array<array<uint>> $feeds` and `$feeds[$total % 2]`, so a declared `// bench: allocations 0` was
  refused at 2.000 and the ns figure was twice the member's: `firstKey` read 59.4 ns/op that way and
  29.4 measured alone. Chain with two plain arrays and a branch, and read a `FAIL` on a declared
  count as a question about the bench before the member. [until: reviewed 2026-09-20]
