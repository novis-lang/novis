Reverses the order of the characters in a string.

`Core\Str::reverse` takes a string and returns a new string with the last character first and the
first character last. The result has the same length as the string you pass in.

A character here is what a person sees as one character. A letter with an accent, an emoji with a
skin tone, and a flag are each one character, so they stay whole and in the right order inside.
For example, "café" becomes "éfac", not a broken string. The empty string gives the empty string.

Reversing a string twice gives the original string back.

**Good to know:** this replaces PHP's `strrev`, which reverses bytes and breaks any character that
is not plain English text.
