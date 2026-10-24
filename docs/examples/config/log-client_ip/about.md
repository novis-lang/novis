Whether the log lines the server writes contain the client's IP address.

The server writes one access line per request when `[log] access` is on. It always writes a line for
a response with a `5xx` status. Each of these lines has a field `clientIp` with the client address.

With `client_ip = false`, these lines have no `clientIp` field. The field is not written as `null`.
The value is `true` or `false`, and it is `true` when it is not set.

Use this setting when you promise your users that you do not store their IP address.
`Core\Request::clientIp()` still returns the address while the request runs, so your program can
limit how many requests one address sends.

Only the person who runs the server can change this setting. A program cannot change it. A new value
is used from the next request after a reload.

The example prints the setting and shows that a program cannot change it.
