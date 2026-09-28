Divides a string into its characters, the way a person counts them.

`Core\Str::graphemes` returns an array of strings, one for each character, in their original order.
`"héllo"` gives `"h"`, `"é"`, `"l"`, `"l"` and `"o"`. An empty string gives an empty array.

One character on the screen can be several code points in the string. An accent can be a separate
mark after its letter, a flag is two code points, and a family emoji can be seven. Each of these
stays one piece. Unicode calls such a character a grapheme. `Core\Str::length` counts the same
pieces, so the array always has `Core\Str::length($s)` elements.

This replaces PHP's `mb_str_split` and `grapheme_str_split`. To get the numbers of the code points
instead, use `Core\Str::codePoints`.

**The examples below** divide a word into its letters, show that a flag and an accent stay whole,
and reverse a name without breaking its accents.
