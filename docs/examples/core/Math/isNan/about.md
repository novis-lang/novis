`Core\Math::isNan` returns `true` when a `float` is `NaN`. `NaN` means "not a number". It is the
result of a calculation that has no answer, such as zero divided by zero with `Core\Math::fdiv`,
or the square root of a negative number.

You cannot find `NaN` with `==`. `NaN` is not equal to any value, and it is not equal to itself
either. So `$x == Core\Math::NAN` is always `false`. Use `Core\Math::isNan` instead.

`INFINITY` and `-INFINITY` are not `NaN`, so `Core\Math::isNan` returns `false` for them.
`Core\Math::isFinite` returns `false` for all three.

**The examples below** show which calculations give `NaN`, then why `==` cannot find it, then a
program that skips the failed readings from a sensor before it works out the average.
