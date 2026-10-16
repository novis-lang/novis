Returns the id of one server-sent event, or `null` when the server has not sent an id yet.

A server writes an id on an `id` line. The id stays in force until the server sends a new one.
So an event without an `id` line has the id of the event before it. An `id` line that contains a
NUL character is ignored.

The id tells your program where the stream stopped. When the connection ends, your program can
connect again and tell the server the last id it received. The server then sends only the newer
events. The id came from another server, so it is `tainted`. Check it before you send it back.

**The examples below** print the id of each event, show an id that stays in force, and continue a
stream after the connection ends.
