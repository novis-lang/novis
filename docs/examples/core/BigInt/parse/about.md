Reads a `Core\BigInt` out of text.

`parse` reads the whole text as a whole number: an optional `-` or `+`, and then digits. There is no
limit on how many digits there are. Nothing else is allowed, so a space, a `_` separator or a `0x`
prefix makes it throw a `ParseError`.

The digits are read in base 10 unless you set the `radix` option. That option may be any base from 2
to 36, where the digits after `9` are the letters `a` to `z`. `Core\BigInt::format` writes a number
back in any base `parse` reads, so a value can be stored as text and read again exactly.

This is how a number that arrived as text becomes a number your program can add or multiply: a field
of a JSON body, a line of a report, a hexadecimal hash. For a number your program already has as an
`int`, use `Core\BigInt::of`.
