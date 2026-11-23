Removes characters from the end of a string.

`Core\Str::trimEnd` returns the string without the spaces, tabs and line breaks at its end. The
start is not changed, so an indented line keeps its indent.

The `characters` option gives your own list of characters to remove. It replaces the default list,
so `{ characters: "/" }` removes only `/`. Each character in the list counts on its own, and the order
does not matter. If every character is in the list, the result is an empty string.

A trim removes whole characters, as a person counts them. Some characters have several code points
(Unicode numbers). Such a character is removed only when all its code points are in the list. So
`Core\Str::trimEnd("e\u{301}", { characters: "\u{301}" })` returns `"e\u{301}"` unchanged. The
default list removes the `"\r\n"` that ends a line a program read, because both its code points are
in it.

`Core\Str::trim` removes characters from both ends, and `Core\Str::trimStart` from the start only.

**The examples below** remove the line break at the end of a line, remove the slashes at the end of
a web address, and show a price without the zeros it does not need.
