Returns the one character at a position in a string.

`Core\Str::at` counts positions from `0`, so position `0` is the first character. A negative
position counts from the end: `-1` is the last character and `-2` the one before it. The result is
always a string with exactly one character in it.

A character here is what a person sees as one letter or symbol. So `"é"` is one character, and so is
an emoji built from several parts. This is the same way `Core\Str::length` counts, so a position
from `0` up to the length minus one always finds a character.

A position outside the string throws a `RuntimeError`. PHP returns an empty string here instead.

**The examples below** read the first and last character, catch a position that is too far, and
build the initials of a person's name.
