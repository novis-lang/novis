Sends one event to the client on an event stream.

The first argument is the data of the event. A string is sent as it is. Any other value, such as an
array or a number, is converted to JSON first. Each line of the data becomes its own `data:` line, and
an empty line ends the event.

Two optional arguments go with the data. `event` is a name: the browser runs the listener added for
that name, and without a name it runs `onmessage`. `id` is sent back by the browser in the
`Last-Event-ID` header when it reconnects, so the server can continue from there.

A name or an id with a line break or a NUL byte throws a `LogicError`, and so does empty data. Nothing
is sent in that case.

**The examples below** send a text, send an array as JSON under a name, and send a shop's new orders
with ids.
