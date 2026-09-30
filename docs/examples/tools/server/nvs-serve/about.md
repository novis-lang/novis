`nvs serve` starts a web server that runs your program one time for each HTTP request.

`nvs serve app.nvs` serves one file at `/`. With no file, it serves every file that the
`[[server.mount]]` blocks in `nvs.toml` name. The server listens on `127.0.0.1:8000`. `[server]
listen` in `nvs.toml`, `--listen` or `--port` sets another address. The server compiles every file
before it accepts a request, so a program with a compile error never answers one. It runs until
you stop it.

Each request starts the program at its first line, with its own memory. A value that one request
stores in a variable is not there for the next request. Data that must stay goes into a session, a
cache or a database.

**In plain words:** each request gets an empty desk. Nothing from the request before is on it.

**Good to know:** development and production use the same command. In production, a proxy in front
of the server does TLS and compression.

**The example below** reads the path and one header of a request. It then counts in a static
variable, which is 0 at the start of every request.
