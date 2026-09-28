Tells the client how long to wait before it reconnects to the event stream.

A browser's `EventSource` reconnects on its own when a stream ends, after a short wait the browser
chooses. `retry()` sends a new wait time to the client, as a `retry:` line. This line is not an
event, so the client's event handlers do not run for it. The time is a `Core\Time\Duration`, and it
is sent in milliseconds. A wait shorter than one millisecond is sent as `0`.

A negative wait is not allowed, and `retry()` throws a `LogicError`. Nothing is sent in that case.

**The examples below** set a wait of five seconds, show the error for a negative wait, and ask
clients to come back later before a server restart.
