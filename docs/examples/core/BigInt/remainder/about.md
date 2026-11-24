Returns what is left over when this number is divided by another one, as a new `Core\BigInt`.

The result has the sign of the number you called it on, which is what `%` does with plain `int`
values. -7 with a divisor of 2 gives -1, and 7 with a divisor of -2 gives 1. The sign of the divisor
is never used.

`dividedBy` gives the quotient. The quotient times the divisor, plus the remainder, is the number you
started with.

A divisor of zero throws an `ArithmeticError`.

**Good to know:** to get a result that is never negative, add `$divisor->abs()` to the result. Then
call `remainder` on that sum with the same divisor.
