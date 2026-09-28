Builds a string from a list of Unicode numbers.

Every symbol in Unicode has a number, called its code point. `Core\Str::fromCodePoints` takes an
array of these numbers and returns one string that contains the symbols in the same order.
`[72, 105]` gives `"Hi"`, and an empty array gives `""`.

It does the opposite of `Core\Str::codePoints`. If you pass a string to `Core\Str::codePoints` and
the result to `Core\Str::fromCodePoints`, you get the same string back.

Every number must be a code point. If one number is larger than `0x10FFFF` or in the reserved range
`0xD800` to `0xDFFF`, the method throws a `RuntimeError`. It does not return the part of the string
that was valid.

This replaces PHP's `mb_chr` called on each number and joined with `implode`.

**The examples below** build a word from its numbers, catch the error for a list with one bad
number, and remove invisible characters from a pasted user name.
