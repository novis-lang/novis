`Core\Test::tlsSession()` builds a TLS session for a test. TLS is the encryption that an `https://`
connection uses. The result is a `Core\Http\TlsInfo`, the same object that
`Core\Http\Response::tls()` returns for a real connection. It has a real certificate chain with two
certificates: the server's certificate, and the certificate that signed it.

Every option has a default, so `tlsSession()` alone gives a TLS 1.3 session for
`CN=example.com` whose certificate was checked. You can choose the version, the cipher, whether the certificate was checked, the
names in the certificates and the expiry date. A value that a real certificate cannot have throws a
`LogicError`.

You can call `tlsSession` in a test and in a command-line program. In a server, it throws a
`LogicError`. This includes a request, a connection, a scheduled task and a queued job. Code that a
test starts may still call it, such as a request sent with `Core\Test::request`.

**The examples below** show the default session, a fake reply that reports a session, and a test
of a client that accepts only TLS 1.3.
