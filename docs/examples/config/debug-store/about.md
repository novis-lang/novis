Saves a record of each request. You look at the records later with `nvs debug`.

With `store = true`, Novis writes a record of each request to files in `debug/spool` in the data
folder. A record has the result, the log lines, each value from `Core\Debug::dump` and the time of
each query. `nvs debug` reads these files into `debug/index.sqlite`. A mail block with
`capture = true` also writes each whole message there, with its attachments.

Jobs from `[[schedule]]` and the queue are recorded too, and so is each run of `nvs run` and
`nvs test`. To record only the server, write `store = false` in a second file, `off.toml`. Pass
it after `nvs.toml`: `nvs run --config nvs.toml --config off.toml`.

In development mode, a record also has the request and the response. The `Authorization` and
`Cookie` headers are replaced with `[redacted]`. In production mode, a record has no headers and no
body.

The default is `false`. A program can turn recording off for its own request. It cannot turn it on.

A body that the program reads as plain text is saved as it was sent, with any password in it. Only
your account can open the record files.
