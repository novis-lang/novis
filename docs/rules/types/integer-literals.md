An integer literal is written decimal, `0x`, `0o` or `0b` — either case of the prefix letter, with `_`
separators allowed between digits. Those four are the closed set, so **a leading zero is not a
radix**: `017` is decimal seventeen, and `0o17` is the only spelling of octal fifteen. A leading zero
that switched the radix would change a value without changing a character, and a file-mode constant
is where that bites.

**There is no literal suffix, for any numeric type.** A literal takes `int`, `uint`, `float` or
`decimal` from the position it is written in instead (`rule:types/numeric-literal-placement`), and a
literal too wide for `int` is legal only where a `uint` is expected.

The escape grammar inside a string literal is unrelated to this, and includes `\v`, `\f`, `\e` and
the octal `\0`–`\777`.
