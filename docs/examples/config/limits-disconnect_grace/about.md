How long a request with no `wall_time` may run after its client went away.

A client can close its connection before it gets the answer. The request still runs to its end,
and the server throws the answer away. A request with a `wall_time` stops at its `wall_time`. A
request with no `wall_time` stops when `disconnect_grace` has passed since the client went away.

The default is `30s`. `false` and `0` are not allowed, so a request always has a limit. Only the
person who runs the server sets this value. A request cannot change it while it runs.

**The example below** reads the value and tries to change it.
