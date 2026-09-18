Everything about the HTTP server itself: where it listens, how it finds the code for a request, and
how long it waits.

One block holds the addresses or Unix sockets to accept on, the directory every mount must resolve
inside, whether a request is routed by entry point or by path, whether static files are served at
all, which proxies are believed about a client's address, the in-flight ceiling, how many cores
accept, four idle waits, and how long a connection keeps being served once the server has begun
stopping.

**In plain words:** this is the shape of the front door. With nothing written, it is a loopback
address on port 8000 — the proxied shape and the development one at once.

The whole block needs a restart to change, and that is a property rather than an omission: the
header and keep-alive waits apply before any of this deployment's code exists on the connection, so
promising that a running server could move them would be promising something the block cannot keep.
A reload over an edited block therefore names the key and carries the running value forward.

The example prints what this server listens on and how long it waits, and is turned away moving any
of it.
