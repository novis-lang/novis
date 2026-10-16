Returns the name of one server-sent event, or `null` when the event has no name.

A server can give each event a name on an `event` line, for example `order.created` or `ping`.
Your program uses the name to decide what to do with the event. The name belongs to one event
only. The next event has no name unless it has its own `event` line. An event without a name is
what the format calls a `message`, so `$event->name() ?? "message"` gives the name in every case.

The name came from another server, so it is `tainted`, like the data.

**The examples below** print the name of each event, show an event without a name, and handle
each kind of event from a build server in its own way.
