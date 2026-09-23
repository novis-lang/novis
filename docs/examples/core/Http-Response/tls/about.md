Returns the details of the TLS connection a reply came over, or `null` when the reply did not come
over TLS.

TLS is the encryption that `https` uses. When a reply arrives over TLS, `tls()` returns a
`Core\Http\TlsInfo`. It has the TLS version, the cipher, the server's certificates, and
`verified()`, which says whether the certificate was checked. A reply over plain `http` has no TLS
connection, so `tls()` returns `null`. The return type is `?Core\Http\TlsInfo`, so your program
checks for `null` before it reads anything.

**Good to know:** the examples use `Core\Test::answerHttp` to give fixed replies, so they run
without a network. A fixed reply does not use any connection, so `tls()` returns `null` in all of
them. They show the check for `null`, a log line with the TLS version, and a program that sends a
token only after a verified connection.
