Removes characters from the start of a string.

`Core\Str::trimStart` returns the string without the spaces, tabs and line breaks at its start. The
end of the string is not changed. This is useful when the end of a text matters, for example
the spaces that end a line of a fixed-width table.

The `characters` option gives your own list of characters to remove. It replaces the default list,
so `{characters: "0"}` removes only zeros. Each character in the list counts on its own, and the
order does not matter. When every character of the string is in the list, the result is the empty
string.

`Core\Str::trim` removes characters from both ends, and `Core\Str::trimEnd` from the end only.

**The examples below** remove spaces at the start of a line, remove the zeros before a number, and
read the headings of a Markdown document.
