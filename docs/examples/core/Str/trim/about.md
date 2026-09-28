Removes characters from both ends of a string.

`Core\Str::trim` returns the string without the spaces, tabs and line breaks at its start and its
end. The characters in the middle of the string are not changed. Text that people type, or that
comes from a file, often has extra spaces like these.

The `characters` option gives your own list of characters to remove. It replaces the default list,
so `{characters: "-"}` removes only `-`. Each character in the list counts on its own, and the order
does not matter. `"a..z"` is the three characters `a`, `.` and `z`. It is not a range of letters.

`Core\Str::trimStart` removes characters from the start only, and `Core\Str::trimEnd` from the end
only. This replaces PHP's `trim`.

**The examples below** clean up text that somebody typed, remove quotes and dashes, and read the
settings from a configuration file.
