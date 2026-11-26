Saves a record of each request, so that you can look at it later with `nvs debug`.

With `store = true`, Novis writes a record of each request to files in `debug/spool` in the data
folder. A record has the result, the log lines, the `Core\Debug::dump` values and the time of each
database query and HTTP call. `nvs debug` reads these files into `debug/index.sqlite` and deletes
them. In development mode, `nvs serve` starts `nvs debug` for you.

A server records each request, scheduled run and queue job. `nvs run` and `nvs test` record each
run too. To record the server and not a command, give the command a second configuration file:
`nvs run --config nvs.toml --config off.toml job.nvs`, where `off.toml` has `store = false` under
`[debug]`.

In development mode, a record also has the request and the response, with their headers and body.
The values of the `Authorization`, `Cookie`, `Set-Cookie` and `Proxy-Authorization` headers are
replaced with `[redacted]`. In production mode, a record has no headers and no body.

The default is `false`. A program cannot turn it on. A program can turn it off for its own request
with `Core\Config::set('debug.store', 'false')`.

**Good to know:** a password in a body that the program reads as plain text is saved in the record.
The files are in a folder that only your account can open.
