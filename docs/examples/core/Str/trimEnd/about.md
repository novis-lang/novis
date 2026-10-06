Removes characters from the end of a string.

`Core\Str::trimEnd` returns the string without the spaces, tabs and line breaks at its end. The
start of the string is not changed, so an indented line keeps its indent. A common use is to remove
the line break at the end of a line that a program read.

The `characters` option gives your own list of characters to remove. It replaces the default list,
so `{characters: "/"}` removes only `/`. Each character in the list counts on its own, and the order
does not matter. When every character of the string is in the list, the result is the empty string.

`Core\Str::trim` removes characters from both ends, and `Core\Str::trimStart` from the start only.

**The examples below** remove the line break at the end of a line, remove the slashes at the end of
a web address, and show a price without the zeros it does not need.
