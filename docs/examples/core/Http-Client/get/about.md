Sends a `GET` request to another server and returns its answer.

A `GET` request reads something from a server, for example a page, a file or data from an API. The
result is a `Core\Http\Response`. `status()` gives the status code, and `text()` gives the body as a
string. The body comes from another server, so it is `tainted` until your program checks it.

The URL is checked before anything is sent. The host must be allowed by `net.connect` in
`[capabilities.net]` in `nvs.toml`, and an address inside your own network throws a `RuntimeError`.
`headers` sets the request headers, and a header with a line break throws a `RuntimeError`. Every
request has a time limit, `deadline`.

**Good to know:** the examples run without a network, so every request in them is refused before it
is sent. They show one refused address, a header with a line break, and a shop that uses a saved
exchange rate when the rate service cannot be reached.
