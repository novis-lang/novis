Returns this number as a `uint`, when a `uint` can hold it.

A `uint` holds whole numbers from 0 to 18446744073709551615. It reaches one bit further up than an
`int` does, and it holds no negative numbers at all. `toUint` returns the `uint` equal to this
number. When the number is negative, or above that range, `toUint` throws an `ArithmeticError`. It
never cuts a number down to fit, so a result you get back is always exact.

Use `toUint` for a value that cannot be negative: a size, a count, a length, an identifier. Where a
number may be negative, test `Core\BigInt::sign` first, or use `Core\BigInt::toInt`, which accepts
negative numbers and stops one bit lower.

**Good to know:** the range of a `uint` covers every 64-bit identifier a database or another service
can give you, which `Core\BigInt::parse` reads from text of any length.
