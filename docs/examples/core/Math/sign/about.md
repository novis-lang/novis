`Core\Math::sign` checks whether a number is below zero, zero, or above zero.
It returns `-1`, `0` or `1`.

It works with `int`, `float` and `decimal` values, and the result is always an `int`. `-0.0` is zero,
so the result for it is `0`. The infinities are not zero: `-Core\Math::INFINITY` gives `-1`.

`NaN` (a value that means "not a number") is not below, above or equal to zero. For `NaN`,
`Core\Math::sign` throws a `RuntimeError`. A `uint` bigger than `Core\Math::INT_MAX` throws an
`ArithmeticError`.

**The examples below** show the three results for each type of number, then zero and `NaN`, then a
report that says whether sales went up or down.
