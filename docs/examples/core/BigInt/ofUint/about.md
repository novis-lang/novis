Builds a `Core\BigInt` from a plain `uint`.

A `uint` is a whole number that is never negative, between 0 and 18446744073709551615. The upper half
of that range is larger than any `int`, so `Core\BigInt::of` cannot be given those numbers at all.
`ofUint` takes the whole range, and every value converts exactly.

Use it for the numbers your program already has as a `uint`: a length, a count of bytes, an
identifier, a value that came out of a bit shift. Once the number is a `Core\BigInt` it can grow past
the `uint` range as well, which is what a total of many counts does.

For a number that may be negative, use `Core\BigInt::of`. For one your program has as text, use
`Core\BigInt::parse`.
