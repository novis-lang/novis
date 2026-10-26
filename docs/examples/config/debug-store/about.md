Saves a record of each request, so that you can look at it later with `nvs debug`.

With `store = true`, Novis writes a record of each request to files in `debug/spool` in the data
folder. A record has the result of the request, its log lines, its `Core\Debug::dump` values and how
long its database queries and HTTP calls took. `nvs debug` reads these files into one database,
`debug/index.sqlite`, and then deletes them. In development mode, `nvs serve` starts `nvs debug` for
you. In production mode, you start it yourself.

In development mode, a record also has the method, path, query, headers and body of the request. It
also has the status, headers and body of the response. The values of the `Authorization`, `Cookie`,
`Set-Cookie` and `Proxy-Authorization` headers are replaced with `[redacted]`. A body is saved as
the program read it, up to `[limits] request_body`. In production mode, a record has no headers and
no body.

The default is `false`. A program cannot turn it on. A program can turn it off for its own request
with `Core\Config::set('debug.store', 'false')`. That request then writes no record.

**Good to know:** a password in a body that the program reads as plain text is saved in the record.
The files are in a folder that only your account can open.
