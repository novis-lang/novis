Finds the greatest common divisor of two whole numbers. This is the largest number that divides
both of them with no remainder. For `12` and `18`, the result is `6`.

The signs of the two numbers do not change the result, and the result is never negative. When one
number is `0`, the result is the other number without its sign. When both are `0`, the result is
`0`. The positive value of `Core\Math::INT_MIN` is too big for an `int`, so a call whose result
would be that value throws an `ArithmeticError`. This replaces PHP's `gmp_gcd`.

Use it to write a fraction or a ratio with the smallest numbers. `Core\Math::lcm` finds the
smallest number that both numbers divide.

**The examples below** show a few pairs of numbers, then zero and negative numbers, then how to
write the size of an image as a short ratio, such as `16:9`.
