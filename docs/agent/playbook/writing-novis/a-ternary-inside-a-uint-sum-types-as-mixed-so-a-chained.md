- **A ternary inside a `uint` sum types as `mixed`, so a chained accumulator needs an `if`.**
  `$total = $total + ($form == Core\Cldr\PluralCategory::Other ? 1 : 3)` fails twice, `E0407`
  mixed-signedness and then `E0401` expected `uint`, found `mixed`, because the two branches are
  `int` literals and the ternary never narrows to the `uint` on the left. Write a `uint $step = 1;`
  with an `if` above the sum instead, which is also what keeps the member's own answer in the chain
  a bench and an attack are built around. [until: reviewed 2026-09-21]
