Builds a `Core\BigInt` from a plain `int`.

A `Core\BigInt` holds a whole number of any size. An `int` holds a whole number between
-9223372036854775808 and 9223372036854775807. `of` moves a number from the second kind to the first,
and every `int`, including both ends of that range, converts exactly.

This is where most `Core\BigInt` values start. You build one from a number your program already has,
then add, multiply or shift it. The result grows as large as it needs to, and an `int` would throw an
error there instead.

For a number your program has as text, use `Core\BigInt::parse`. For one that fits a `uint` but not
an `int`, use `Core\BigInt::ofUint`.
