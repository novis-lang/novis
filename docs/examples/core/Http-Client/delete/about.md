Sends a `DELETE` request to another server and returns its answer.

A `DELETE` request asks a server to remove something, for example an order or a webhook. The result
is a `Core\Http\Response`, and `status()` gives the status code the server sent: `204` or `200` when
it removed the item, `404` when the item was already gone.

The URL is checked before anything is sent. The host must be allowed by `net.connect` in
`[capabilities.net]` in `nvs.toml`, and an address inside your own network throws a `RuntimeError`.
Every request has a time limit, `deadline`. A `DELETE` can be sent again safely, so `retryAttempts`
may try it more than once.

**Good to know:** the examples run without a network, so every request in them is refused before it
is sent. They show one refused address, the two limits, and a job that removes a customer's webhooks.
