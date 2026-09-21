Replaces every row a live region shows.

`Core\Cli::live` opens a region on the terminal and hands your code a `Core\Cli\Live`. `set` is the
one thing you can do with it: you give it the rows the region should show now, one `Core\Cli\Text`
each, and they replace the rows it showed before. Nothing is added to a region and nothing scrolls
past. Give it fewer rows and the region becomes smaller; give it an empty array and it shows
nothing.

A row is a `Core\Cli\Text`, which `Core\Cli\Text::plain` makes from a string and
`Core\Cli\Text::styled` makes with a colour. A row wider than the terminal is cut at the edge
rather than wrapped onto a second line, so the region keeps its shape. Where the output is not a
terminal, such as a log file, `set` draws nothing at all and your code still runs.

**The examples below** replace one row with another, give a row a colour, and show a table of
transfers that becomes smaller as they finish.
