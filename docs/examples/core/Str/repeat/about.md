Returns a string written a number of times, one copy after the other.

`Core\Str::repeat` takes a string and a count, and returns one string that contains that many
copies: `repeat("ab", 3)` gives `"ababab"`. A count of `0`, or an empty string, gives `""`.

The count is a `uint`, so it can never be negative. A very large count can ask for more memory
than the program may use. Then the program stops at its memory limit with an error. A result
larger than any process can hold throws a `RuntimeError` instead.

Programs use it to draw lines and bars in a terminal, and to indent text by a depth.

**Good to know:** this replaces PHP's `str_repeat`.
