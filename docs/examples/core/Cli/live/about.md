Keeps a block of lines on the screen up to date, in place.

A program that reports its progress usually needs one line that changes, not a hundred lines that
scroll past. `Core\Cli::live` opens a region on the terminal and runs your code with a handle to it.
`set` gives the region its rows, and every call replaces all of them, so the picture is drawn again
rather than added to. A row is a `Core\Cli\Text`, which `Core\Cli\Text::plain` makes from a string.

The region closes when `Core\Cli::live` returns, and it closes on every way out: a return, an error,
or the program stopping. The terminal is left the way it was found. Where the output is not a
terminal, a log file for example, the region draws nothing at all and your code still runs.

`Core\Cli::live` returns exactly what your code returned, at its own type.

**The examples below** keep one line up to date, show several rows at once, and work through a list
of jobs.
