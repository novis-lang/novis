Sends a `POST` request to another server and returns its answer.

A `POST` request sends data to a server, for example a new order. You choose one option for the
data. `json` sends a value as JSON. `form` sends fields like an HTML form. `body` sends bytes
unchanged. The result is a `Core\Http\Response`, and `status()` gives the status code.

The URL is checked before anything is sent. Any public host is allowed, unless `net.connect` in
`[capabilities.net]` lists the allowed hosts. An address inside your own network throws a
`RuntimeError`. Loopback (`127.0.0.1`) is allowed only from the command line and in development mode.

A `POST` that is sent twice can create two orders. So `retryAttempts` on a `POST` also needs
`retryIdempotencyKey`. Without it, the program does not compile. The key is sent with every attempt,
so the server can recognise a repeated request.

**Good to know:** the examples run without a network, so every request in them fails.
