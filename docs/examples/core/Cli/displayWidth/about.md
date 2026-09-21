Counts how many terminal columns a text needs.

A terminal writes text into a grid of columns. Most characters take one column. A Chinese, Japanese
or Korean character takes two, an emoji takes two, and a combining mark takes none.
`Core\Cli::displayWidth` counts the columns the terminal will use, so your program can pad a table
column, draw a box or shorten a line and have it come out straight. Counting characters instead makes
a table of mixed scripts come out crooked.

Two characters are measured by what they do to the cursor. A tab moves to the next column that is a
multiple of eight. A newline ends the row, and the result is then the width of the widest row.

**Good to know:** the text may be tainted. A column count is a number, so nothing of what was measured
comes back out of it.

**The examples below** measure a label, compare characters with columns, and line up a table.
