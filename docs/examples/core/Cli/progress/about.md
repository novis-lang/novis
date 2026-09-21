Draws a progress bar on the terminal while your code works.

`Core\Cli::progress` opens a bar scaled to `$total` and runs your code with a handle to it. Your code
counts: `advance` says that one more unit of work is done, and the runtime draws the bar. `{by: 1000}`
counts a thousand units in one step, and `{label: "..."}` sets the caption beside the bar.

The bar closes when `Core\Cli::progress` returns, on every way out, and the terminal is left the way
it was found. Where the output is not a terminal, a log file for example, nothing is drawn and your
code still runs. A count past `$total` reads as finished rather than as more than finished, so a
loop that miscounts draws a full bar. A `$total` of `0` is work already done, and draws a full bar.

`Core\Cli::progress` returns exactly what your code returned, at its own type. `Core\Cli::live` is
the general form, for a region whose rows you write yourself.

**The examples below** count three files, say what is being done, and import rows in batches.
