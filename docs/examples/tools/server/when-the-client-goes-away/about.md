A client can close its connection before it gets the answer. For example, the user closes the
browser tab. The request still runs to its end, and the server throws its answer away.

Every write to the answer returns at once and does not throw an error. This is true for `echo`,
for `Core\Response` and for `Core\Response\Stream::write`. Work that the request gave to
`Core\Task::afterResponse` still runs. So a request that saves an order saves all of it.

A request with a `wall_time` stops at its `wall_time`. A request with no `wall_time` stops when
`[limits] disconnect_grace` has passed since the client went away. The default is `30s`.

`[limits] cancel_on_disconnect` lists request methods, for example `["GET", "HEAD"]`. A request
with a listed method stops as soon as its client goes away. A request that stops runs no more code
of your program, and no `catch` block runs.

**Good to know:** a program cannot test whether its client has gone, and it cannot change these
two settings while it runs. `[app.limits]` can set them for one application.

**The examples below** run under an `nvs.toml` that sets both settings. No client goes away while
they run, so each one prints its whole output.
