`&&` means "both", `||` means "either" and `!` means "not". Each one tests its operands as
conditions and returns `true` or `false`. The result is always a `bool`.

Every value counts as true except these: `false`, `null`, zero, an empty string, the string `"0"`
and an empty array. This is the same list as in PHP, with one difference. A `bytes` value is false
only when it is empty, so a single `0` byte counts as true. An object, a callable and an enum case
are always true.

`&&` and `||` stop when the result is known. The right side runs only when the left side did not
decide the result. In `$count != 0 && $total / $count > 10.0`, the division runs only when `$count`
is not zero.

**Good to know:** `and`, `or` and `xor` are not part of the language. No logical operator returns
one of its operands.

**The examples below** show a check that reads a value only when the value exists, the values of a
form field that count as empty, and the checks a request must pass.
