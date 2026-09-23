Sends a `POST` request to another server and returns its answer.

A `POST` request sends data to a server, for example a new order or a message for an API. You send
the data with one option: `json` sends a value as JSON, `form` sends fields the way an HTML form
does, and `body` sends bytes exactly as you give them. The result is a `Core\Http\Response`, and
`status()` gives the status code the server sent.

The URL is checked before anything is sent. The host must be allowed by `net.connect` in
`[capabilities.net]` in `nvs.toml`, and an address inside your own network throws a `RuntimeError`.
A `POST` that is sent twice can create two orders. So `retryAttempts` on a `POST` also needs
`retryIdempotencyKey`, and without it the program does not compile. The key is sent with every
attempt, and the server uses it to recognise a repeated request.

**Good to know:** the examples run without a network, so every request in them is refused before it
is sent. They show an order sent as JSON, the limits for retries, and a shop that sends a payment.
