Returns the data of one server-sent event, as a string.

A server that sends events writes the data of each event on one or more `data` lines. `data()`
joins those lines with a line break and returns the result. An event always has data. A block
without a `data` line is not an event, so a `foreach` over `Core\Http\Stream::events()` does not
give it to your program.

The data came from another server, so it is `tainted`. You can print it, compare it and join it
with other text. Check it before you use it in a place that needs trusted text.

**The examples below** print the data of each event, show an event with three `data` lines, and
build one long answer from the small pieces a text generation service sends.
