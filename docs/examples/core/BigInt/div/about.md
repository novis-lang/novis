Returns the whole part of this number divided by another one, as a new `Core\BigInt`.

The fraction is dropped. A positive and a negative result are both cut toward zero, so 7 divided by
2 is 3, and -7 divided by 2 is -3. That is the same direction `Core\Math::intDiv` cuts a plain `int`
division.

Dividing by zero throws an `ArithmeticError`. Check the divisor with `sign` first when it may be
zero.

Both numbers stay as they were. `div` returns a new number, so the result is the number you get
back, not the one you called it on. To get what the division left over, use `mod` on the same two
numbers.
