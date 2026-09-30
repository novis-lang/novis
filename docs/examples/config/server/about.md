Settings for the HTTP server: where it listens, how it finds the code for a request, and how long
it waits.

The block sets the addresses or Unix sockets to listen on and the directory that every mount must
be in. It also sets which proxies are trusted, how many requests run at once, the number of workers
and the timeouts. When nothing is set, the server listens on `127.0.0.1`, port `8000`.

When `[mode] default` is `development`, the server runs the `.nvs` file at the path in the URL and
serves static files. In production mode, every request runs the entry file of the mount, and no
static file is served. Your own `dispatch` or `static` value replaces these defaults.

**Good to know:** the server applies a change to this block while it runs. `listen`, `socket_mode`
and `workers` need a restart. A program cannot change any setting in this block.

The example prints the address, the header timeout and the trusted proxies. It then tries to change
them, and `Core\Config::set` returns `false` each time.
