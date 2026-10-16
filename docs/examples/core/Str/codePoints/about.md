Lists the numbers that Unicode gives to each symbol in a string.

Every symbol in Unicode has a number, called its code point. `"A"` is `65`, `"é"` is `233` and `"€"`
is `8364`. `Core\Str::codePoints` returns these numbers as an array of `uint`, in order. An empty
string gives an empty array.

A code point is not always a whole character. An accent can be its own code point, placed after a
letter, and a flag is two code points. So `"é"` can give one number or two, depending on how it
was written. To count or divide text the way a person sees it, use `Core\Str::length` or
`Core\Str::graphemes`.

`Core\Str::fromCodePoints` does the opposite and builds a string from the numbers.

**The examples below** list the numbers of a short word, show one accented letter written in two
ways, and find an invisible character in a pasted user name.
