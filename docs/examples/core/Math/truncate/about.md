`Core\Math::truncate` removes the fractional part of a number. The result moves toward zero: `2.7`
gives `2.0`, and `-2.7` gives `-2.0`. `Core\Math::floor` is different for negative numbers: it gives
`-3.0` for `-2.7`.

The result is still a `float`. To get an `int`, convert it with `as int`. A number between `-1.0`
and `0.0` gives `-0.0`, which is equal to `0.0`. `NaN` (a value that means "not a number") and the
infinities stay the same. Very large `float` values have no fractional part, so they also stay the
same. `Core\Math::truncate` never throws an error.

**The examples below** show positive and negative numbers, then the special values, then a timer
that shows whole seconds.
