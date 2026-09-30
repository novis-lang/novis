When the server is told to stop, it first finishes the work it has. This is called the drain.

A stop is `Ctrl-C`, `nvs service stop` or a stop from the service manager. From that moment the
server accepts no new connection. A request that is running is answered in full, and then its
connection closes. A connection with no work closes at once. A WebSocket client gets the close
code `going away`, so it can connect again. When every connection is closed, the server exits.

A program reads the state with `Core\Server::isDraining()`, which returns `true` after the stop.
A long request can test it and end early.

**Good to know:** `nvs ctl reload` does not drain. It closes no connection, and a request that is
running finishes with the configuration it started with.

**The example below** shows a loop that tests `Core\Server::isDraining()` before each step.
