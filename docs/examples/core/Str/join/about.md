Joins a list of strings into one string.

`Core\Str::join` takes an array of strings and a separator. It returns one string that has the
separator between each two neighbouring strings, in the order of the array. The separator is not
added before the first string or after the last one. If you leave out the separator, the strings
are joined with nothing between them.

An empty array gives the empty string `""`. An array with one string gives that string. An empty
string in the array still gets a separator on each side, so `["a", "", "b"]` joined with `","`
gives `"a,,b"`.

`Core\Str::split` does the opposite: it cuts one string into a list at a separator.

**Good to know:** this replaces PHP's `implode`. The array always comes first, and the separator
second.
