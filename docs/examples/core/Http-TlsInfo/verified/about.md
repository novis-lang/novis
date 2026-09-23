Returns `true` when the server's certificate was checked for this connection, and `false` when it
was not.

When a program calls an `https` address, the client checks two things. The certificate must be
signed by an authority the client trusts, and it must be for the server name in the address. Only
when both checks run does `verified()` return `true`. A configuration can turn a check off for one
server, for example a test server with a self-signed certificate. `verified()` is how a program or
a test sees that: every other call should still return `true`.

You get a `Core\Http\TlsInfo` from `Core\Http\Response::tls()`. That method returns `null` for a
reply that did not come over TLS, so check for `null` first.

**Good to know:** the examples build their TLS connections with `Core\Test::tlsSession`, so they
run without a network. They show the check itself, a warning line, and an audit that allows only
one server to skip the check.
