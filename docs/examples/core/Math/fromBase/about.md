`Core\Math::fromBase` reads a number written in another base and returns it as an `int`. The base
is from `2` to `36`. Base 2 is binary, base 16 is hexadecimal, and digits above `9` are the letters
`a` to `z`. Upper and lower case letters are the same digit. This replaces PHP's `bindec`, `hexdec`,
`octdec` and `base_convert`.

A leading `-` makes the number negative. Every other character must be a digit of the base. A
prefix such as `0x`, a space or an empty string throws a `RuntimeError`. PHP skips a character it
cannot read. A number too large for an `int` throws an `ArithmeticError`.

`Core\Math::toBase` does the opposite. It writes an `int` in a base.

**The examples below** show binary and hexadecimal numbers, then text that is not a number, then how
to read a colour code such as `#FF8800`.
