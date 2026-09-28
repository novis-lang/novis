Splits a text into its lines.

`Core\Str::lines` takes one string and returns an array with one string for each line. The line
breaks are not part of the result. A line ends at `\n`, at `\r\n` or at a single `\r`. All three
work on every system, so a text written on Windows gives the same lines as one written on Linux.

A line break at the very end of the text does not add an empty line, so `"a\nb\n"` gives two
lines. An empty line in the middle is kept: `"a\n\nb"` gives three lines, and the second one is
`""`. An empty string gives an empty array.

`Core\Str::join` with `"\n"` does the opposite: it puts a list of lines back into one text.

**Good to know:** this replaces PHP's `explode(PHP_EOL, $s)`. `PHP_EOL` depends on the system, and
`Core\Str::lines` does not.
