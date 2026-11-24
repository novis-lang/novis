Replaces every row a live region shows.

`Core\Cli::showLive` opens a region on the terminal. A region is a block of rows that is drawn again
in place. `showLive` gives your code a `Core\Cli\Live`, and `set` is its only method. You pass the
rows the region shows now, one `Core\Cli\Text` each. They replace the rows it showed before.
The region does not grow, and the terminal does not scroll. With fewer rows, the region becomes
smaller. With an empty array, it shows nothing.

A row is a `Core\Cli\Text`, which `Core\Cli\Text::plain` makes from a string and
`Core\Cli\Text::styled` makes with a colour. A row wider than the terminal is cut off at the last
column. It is not wrapped onto a second line, so the region keeps its shape. Where the output is not a
terminal, such as a log file, `set` draws nothing at all and your code still runs.

**The examples below** replace one row with another, give a row a colour, and show a table of
transfers that becomes smaller as they finish.
