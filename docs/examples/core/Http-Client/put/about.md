Sends a `PUT` request to another server and returns its answer.

A `PUT` request sends a whole record to a server, and the server saves it at that URL. If a record
is already there, the new one replaces it. You give the record with one body option: `json`, `form`,
`body` or `multipart`. The result is a `Core\Http\Response`, and `status()` gives the status code.

The URL is checked before anything is sent. Any public host is allowed, unless `net.connect` in
`[capabilities.net]` in `nvs.toml` lists the allowed hosts. An address inside your own network
throws a `RuntimeError`. Loopback (`127.0.0.1`) is allowed only from the command line and in
development mode.
A `PUT` request sent twice gives the same result, so `retryAttempts` can try it again with no other
option. `retryBackoff` is the wait between two tries.

**Good to know:** the examples run without a network, so every request in them is refused before it
is sent. They show one refused address, the wait between two tries, and a job that saves stock
counts on a warehouse server.
