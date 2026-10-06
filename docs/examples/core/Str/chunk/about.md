Divides a string into pieces that each have the same number of characters.

`Core\Str::chunk` returns an array of strings in their original order. Every piece has the size you
give, except the last one, which has what is left. Dividing `"abcdefg"` into pieces of `3` gives
`"abc"`, `"def"` and `"g"`. An empty string gives an empty array.

The size counts characters the way a person sees them. An accented letter or an emoji is one
character, so a piece never cuts one in half. The size must be at least `1`. A size of `0` throws a
`RuntimeError`.

**The examples below** divide a word into pairs of letters, show that accents and emoji stay whole,
and format a long card number in groups of four digits.
