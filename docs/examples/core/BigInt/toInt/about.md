Returns this number as an `int`, when an `int` can hold it.

A `Core\BigInt` has no size limit. An `int` does: it holds whole numbers from -9223372036854775808
to 9223372036854775807. `toInt` returns the `int` equal to this number, with the same sign and the
same value. When the number is outside that range, `toInt` throws an `ArithmeticError`. It never
cuts a number down to fit, so a result you get back is always exact.

This is how a large calculation comes back to a plain number, for a count, a loop or a column in a
database. Catch the `ArithmeticError` wherever the number may be too large, and use
`Core\BigInt::toString` instead where you only need to print it.

**Good to know:** `Core\BigInt::of` goes the other way and accepts every `int`, so a number that
came from an `int` and only grew smaller always narrows again.
