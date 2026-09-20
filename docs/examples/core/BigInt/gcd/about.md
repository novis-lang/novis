Returns the greatest common divisor of this number and another one, as a new `Core\BigInt`.

The greatest common divisor is the largest number that divides both of them exactly. It is what you
use to reduce a fraction or a ratio to its shortest form: divide both sides by it.

The result is never negative, and the sign of the two numbers does not change it. The order of the
two numbers does not change it either. When one of them is zero the result is the other number,
because every number divides zero exactly. Two zeroes give 0.

**Good to know:** `Core\Math::gcd` does the same for plain `int` values. Use `Core\BigInt::gcd` when
a number may grow past what an `int` holds.
