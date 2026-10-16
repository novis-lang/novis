Raises a `Core\BigInt` to a power.

`pow` multiplies the number by itself as many times as the power says. The power is a whole number
and is never negative. The power 0 gives 1 for every number, and the power 1 gives the number back.
A negative number raised to an even power is positive.

The answer grows very quickly: 2 to the power 1000 already has 302 digits. One call may produce a
number of at most 1048576 bits, which is about 315653 digits in base 10. A larger answer throws an
`ArithmeticError`. `pow` tests the size before it multiplies anything, so the number is never built
and your program keeps its memory.

To raise a number to a power and then take the remainder, use `Core\BigInt::powMod`. It is much
faster for a large power, and it never holds the whole answer at once.
