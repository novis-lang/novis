Splits a text into pieces at every match of a pattern, and returns the pieces in an array. The
matches themselves are not part of any piece. If there is no match, the array has one piece: the
whole text.

Two matches next to each other give an empty piece. The `keepEmpty: false` option removes the
empty pieces. The `limit` option sets the largest number of pieces, and the last piece then
contains the rest of the text. A negative `limit` removes that many pieces from the end, and `0`
returns the whole text as one piece. This replaces PHP's `preg_split`, which reads `0` and `-1` as
no limit.

**Good to know:** to split at one fixed text, such as a comma, `Core\Str::split` is simpler.

The examples show a split at commas, the `limit` and `keepEmpty` options, and reading the words of
a search that a person typed.
