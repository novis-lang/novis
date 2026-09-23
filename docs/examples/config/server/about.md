Settings for the HTTP server itself: where it listens, how it finds the code for a request, and how
long it waits.

The block has the addresses or Unix sockets to listen on, the directory every mount must be inside,
whether a request is routed by entry file or by path, and whether static files are served. It also
has the proxies whose forwarding headers are trusted, the limit on requests in progress, the number
of workers, four idle timeouts, and how long the server keeps serving while it stops.

**In plain words:** this is the front door of your application. When nothing is set, the server
listens on `127.0.0.1`, port `8000`.

The server applies a change to this block while it runs. Three settings need a restart: `listen`,
`socket_mode` and `workers`. A program cannot change any setting in this block.

The example prints the address, the header timeout and the trusted proxies. Then it tries to change
them, and `Core\Config::set` returns `false` each time.
