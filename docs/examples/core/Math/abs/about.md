Returns a number without its sign, in the same type as the number you give it.

A negative number becomes positive. A positive number and zero stay the same. An `int` gives an
`int`, a `float` gives a `float` and a `decimal` gives a `decimal`, so a price keeps its decimal
places. This replaces PHP's `abs`.

Use it when only the size of a number matters, not its direction: how far apart two readings are,
or how much money is owed, whichever side owes it.

**Good to know:** the smallest `int` has no positive `int` of the same size, so `abs` throws an
`ArithmeticError` for it. `Core\BigInt::abs` has no such limit.
