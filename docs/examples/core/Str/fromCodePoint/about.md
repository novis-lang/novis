Builds a string of one symbol from its Unicode number.

Every symbol in Unicode has a number, called its code point. `Core\Str::fromCodePoint` takes that
number as a `uint` and returns a string that contains the symbol. `72` gives `"H"`, `233` gives
`"é"` and `8364` gives `"€"`.

Not every number is a code point. The largest one is `0x10FFFF`. The numbers from `0xD800` to
`0xDFFF` are reserved, and Unicode gives them no symbol. For these numbers the method throws a
`RuntimeError`.

This replaces PHP's `mb_chr`. `Core\Str::codePoints` does the opposite and returns the numbers of a
string. To build a string from many numbers at once, use `Core\Str::fromCodePoints`.

**The examples below** build a few symbols from their numbers, catch the error for a number that is
not a code point, and turn a country code into its flag.
