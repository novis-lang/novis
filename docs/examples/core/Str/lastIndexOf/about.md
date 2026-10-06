Finds where a piece of text appears for the last time in a string.

`Core\Str::lastIndexOf` searches `$haystack` for `$needle` and returns the position of the last
match. The first character is position `0`. When the needle is not in the string, the result is
`null`. Matches may overlap, so searching `"banana"` for `"ana"` gives `3`.

The position counts characters the way a person sees them, so an accented letter or an emoji counts
as one. You can give the position to `Core\Str::slice` to cut the text at the match.

Two options change the search. `before` searches only the characters in front of that position, so
a match must end there or earlier. A negative `before` counts back from the end of the string.
`caseInsensitive: true` treats upper-case and lower-case letters as the same.

**The examples below** find the last match in a file path, use both options, and shorten a long
text at the last space.
