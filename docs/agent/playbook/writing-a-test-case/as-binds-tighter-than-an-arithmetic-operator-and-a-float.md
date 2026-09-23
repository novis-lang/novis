- **`as` binds tighter than an arithmetic operator, and a `float` that is not whole refuses to
  become an `int`.** `$minutes / 60 as int` parses as `$minutes / (60 as int)` and is `int|float`,
  and the parenthesised `($minutes / 60) as int` then throws `cannot convert this value to int` for
  every quotient with a fraction. There is no `Core\Math::intdiv`, so a proof program that wants
  whole-number division keeps the value in `int` arithmetic (`%` and subtraction) instead.
  [until: reviewed 2026-09-19]
