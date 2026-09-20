Returns the integer square root of this number, as a new `Core\BigInt`.

The square root of a whole number is usually not a whole number itself. `sqrt` returns the largest
whole number whose square is not larger than this one. The square root of 8 is between 2 and 3, so
`sqrt` returns 2. What comes after the decimal point is dropped, and the result is never rounded up.

A negative number has no integer square root. `sqrt` throws an `ArithmeticError` when you call it on
one, so check the sign first where a number may be negative.

**Good to know:** the result is exact at every size. Multiply it by itself and compare that with the
number you started from, and you know whether that number is a perfect square.
