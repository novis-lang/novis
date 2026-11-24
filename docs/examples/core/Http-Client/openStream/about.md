Sends a request to another server and lets your program read the answer while it is still arriving.

The call returns as soon as the status and the headers have arrived. The result is a
`Core\Http\Stream`: `status()` and `header()` read them, and one of four methods reads the body.
`events()` gives server-sent events, `lines()` gives one line at a time, `chunks()` gives the bytes,
and `saveTo()` writes the body to a file. The body can be read only once.

You choose the method, for example `Core\Http\Method::Post`. The URL is checked before anything is
sent. Two more options limit a slow body: `idle` is the longest pause between two pieces, and
`maxDuration` is the most time the whole body may take.

**Good to know:** the examples run without a network, so every request in them is refused before it
is sent. They show a feed of live events, the two time limits, and a job that imports orders one
line at a time.
