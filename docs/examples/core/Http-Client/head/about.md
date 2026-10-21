Sends a `HEAD` request to another server and returns its status and headers.

A `HEAD` request asks the same question as a `GET` request, but the server sends no body. You use it
to learn whether a page or a file exists, or how large a file is, without downloading it. The result
is a `Core\Http\Response`: `status()` gives the status code, `header()` gives one header, and the
body is always empty.

The URL is checked before anything is sent. Any public host is allowed, unless `net.connect` in
`[capabilities.net]` in `nvs.toml` lists the allowed hosts. An address inside your own network
throws a `RuntimeError`. Loopback (`127.0.0.1`) is allowed only from the command line and in
development mode.
`connectTimeout` limits the wait for the connection, and `deadline` limits the whole request.

**Good to know:** the examples run without a network, so every request in them is refused before it
is sent. They show one refused address, the time limit for connecting, and a link checker for a blog.
