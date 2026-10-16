Finds the least common multiple of two whole numbers. This is the smallest positive number that
both of them divide with no remainder. For `4` and `6`, the result is `12`.

The signs of the two numbers do not change the result, and the result is never negative. When one
of the numbers is `0`, the result is `0`. When the result is bigger than `Core\Math::INT_MAX`, the
call throws an `ArithmeticError`. This happens quickly for two big numbers that share no divisor.

Use it to find when two things that repeat at different intervals happen together again.
`Core\Math::gcd` finds the largest number that divides both numbers.

**The examples below** show a few pairs of numbers, then a result that is too big for an `int`,
then when two scheduled jobs next run in the same minute.
