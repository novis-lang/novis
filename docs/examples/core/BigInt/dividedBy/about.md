Returns the whole part of this number divided by another one, as a new `Core\BigInt`.

The fraction is dropped. A positive and a negative result are both cut toward zero, so 7 divided by
2 is 3, and -7 divided by 2 is -3. That is the same direction `Core\Math::quotient` cuts a plain
`int` division.

Dividing by zero throws an `ArithmeticError`. Check the divisor with `sign` first when it may be
zero.

`dividedBy` does not change either number. It returns the result as a new number. To get what the
division left over, use `remainder` on the same two numbers.
