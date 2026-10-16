Reads the body of a streamed reply one line at a time.

A `foreach` over `lines()` gives each line as a `tainted string`, because another server sent it.
A line ends at `\n`. A `\r` just before the `\n` is removed too, so both kinds of line ending work.
An empty line is given as an empty string. The last line is given even when it has no line ending.

The body of a stream can be read only once. After `lines()`, a call to `lines()`, `chunks()`,
`events()` or `saveTo()` on the same stream throws a `LogicError`.

**Good to know:** only the current line is kept in memory, so the reply can be much larger than
the memory your program may use. One line may have at most 64 KB. A longer line throws an error.

**The examples below** number the lines of a log, show which line endings are removed, and add up
a column of a large export.
