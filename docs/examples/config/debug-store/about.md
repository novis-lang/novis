Saves a record of each request, so that you can look at it later with `nvs debug`.

With `store = true`, Novis writes a record of each request to files in `debug/spool` in the data
folder. A record has the result, the log lines, each `Core\Debug::dump` value and the time of each
query. `nvs debug` reads these files into `debug/index.sqlite`.

A server also records each job of a `[[schedule]]` block and each job from the queue. `nvs run` and
`nvs test` record each run too. To record only the server, write `store = false` under `[debug]` in `off.toml`. Then
run commands with `nvs run --config nvs.toml --config off.toml`.

In development mode, a record also has the request and the response. The values of the
`Authorization` and `Cookie` headers are replaced with `[redacted]`. In production mode, a record has
no headers and no body.

The default is `false`. A program can turn recording off for its own request. It cannot turn it on.

A body that the program reads as plain text is saved as it was sent, with any password in it. Only
your account can open the record files.
