`Core\Math::remainder` returns the remainder after dividing one `float` by another. For whole
numbers, use the `%` operator.

The remainder has the same sign as the first value. So `Core\Math::remainder(-7.5, 2.0)` is `-1.5`,
and the sign of the second value does not change the result. To get a result that is never negative,
add the absolute value of the second value to the result. Then take the remainder of that sum again.

When the second value is an infinity, the result is the first value. When the first value is an
infinity, the result is `NaN` (a value that means "not a number"). When either value is `NaN`, the
result is also `NaN`.

When the second value is zero, `Core\Math::remainder` throws an `ArithmeticError`.

**The examples below** show remainders with both signs, then zero and infinity, then a compass
heading that stays between 0 and 360 degrees.
