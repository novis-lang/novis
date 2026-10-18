`??` runs its right side only when the left side is `null` or a missing key. The right side may be
`throw`: `$id ?? throw new LogicError("no id")`. A missing key at any level gives the right side, so
`$t["a"]["b"] ?? 7` is `7` when `$t` has no key `"a"`. `??` on a value that can never be `null`
compiles, and the result is the left side.

The type of `$a ?? $b` is the type of `$a` without `null`, joined with the type of `$b`. So an `?int`
`$a` with `"x"` on the right gives `string|int`. An empty array `[]` on the right takes the type the
result needs. Under `var`, it takes the type of the left side. So `$names ?? [1]` with an
`?array<string>` `$names` is the error `E0401`.

`??` runs after `+`. `$n ?? 0 + 10` is `$n ?? (0 + 10)`, and the compiler gives warning `W1022`.
Write `($n ?? 0) + 10`.

`$a ??= $b` runs `$b` only when it writes. Its value is the value of `$a` afterwards, so
`($a ??= 5) + 1` is `6` when `$a` was `null`. The type of `$a` does not need to include `null`.

`??+=`, `??-=` and `??.=` are the only three of their kind. `??*=` is the error `E0105`, and so is
`($a ?? 100) -= 1`. Write `$a = ($a ?? 100) - 1` instead. A missing key starts at the zero of the
array's element type: `0`, `0.0`, a `decimal` zero or `""`. On a `uint`, `??-= 1` from a missing key
throws an `ArithmeticError`, because `0 - 1` does not fit.

`$a ?: $b` runs `$a` once. The type of the result is the type of `$a` joined with the type of `$b`.

`$o?->m($x)` does not run the arguments when `$o` is `null`. In a chain such as `$q?->next?->value`,
the first `null` makes the whole result `null`. A `?->` on a value that can never be `null` works
like `->`. Writing through `?->`, such as `$p?->x = 2`, is the error `E0479`.

`$c ? $a : $b` runs only the arm it picks. The type of the result is the types of both arms joined,
and it is widened when it is stored: `float $x = $c ? 1 : 2.5` is `1.0` or `2.5`. A ternary in the
last arm of another ternary needs no parentheses. `$n > 9 ? "huge" : $n > 3 ? "big" : "small"` tests
from left to right.
