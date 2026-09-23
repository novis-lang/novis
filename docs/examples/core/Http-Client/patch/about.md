Sends a `PATCH` request to another server to change part of a record, and returns its answer.

A `PATCH` request sends only the fields that change, for example the new status of an order. You
usually send them as JSON with the `json` option. The result is a `Core\Http\Response`, and
`status()` gives the status code the server sent. The URL is checked before anything is sent. The
host must be allowed by `net.connect` in `nvs.toml`, and an address inside your own network throws a
`RuntimeError`.

A `PATCH` that is sent twice can change the record twice. So when you set `retryAttempts`, you must
also set `retryIdempotencyKey`, or the program does not compile. The key is sent in the
`Idempotency-Key` header. It is the same on every attempt, so the server can see that a request is a
repeat.

**Good to know:** the examples run without a network, so every request in them is refused before it
is sent.
