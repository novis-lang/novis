Counts work as done on an open progress bar.

`Core\Cli::progress` opens a bar on the terminal and hands your code a `Core\Cli\Progress`.
`advance` is the one thing you can do with it. Each call marks more of the work as finished and
draws the bar again. `by` says how many units this step finished, and is one when you leave it out.
`label` sets the caption beside the bar, and leaving it out keeps the caption that is already there.

The bar is scaled to the total you gave `Core\Cli::progress`. A bar counted past that total shows as
complete. The handle belongs to the call that made it. Use it after that call has returned, or while
a second bar is open inside it, and `advance` throws a `LogicError`. Where the output is not a
terminal, such as a log file, `advance` draws nothing and your program still runs.

**The examples below** count one step at a time, count a whole block of rows in one step, and name
each file a tool is uploading beside the bar.
