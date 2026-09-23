Returns the TLS version of the connection a reply came over, as a string.

TLS is the encryption that `https` uses. The client and the server agree on a version when they
connect. `version()` returns `TLSv1.3` or `TLSv1.2`, which is how most TLS tools write these
versions. TLS 1.3 is the newest version. Some older servers still use TLS 1.2, and a program can
check for that before it sends private data.

You get a `Core\Http\TlsInfo` from `Core\Http\Response::tls()`. That method returns `null` for a
reply that did not come over TLS, so check for `null` first.

**Good to know:** the examples build their TLS connections with `Core\Test::tlsSession`, so they
run without a network. They show how to read the version, how to accept only TLS 1.3, and how to
list the partner servers that still use TLS 1.2.
