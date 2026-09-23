- **A null test on the left of a `&&` does not narrow the operand on its right.** `$name != null &&
  Core\Str::length($name) > 2` is `E0401` at `$name`, because `rule:types/narrowing` is branch-local:
  the narrowing reaches the `if` body, not the operand written beside the test. Write the second
  check in a nested `if`, or reach the value through `?->` or `??` — and when a guard is only wanted
  for a throw, the ordinary `$count != 0 && $total / $count > 10.0` shape needs no narrowing at all.
  [until: gone docs/rules/types/narrowing.md:branch-local]
