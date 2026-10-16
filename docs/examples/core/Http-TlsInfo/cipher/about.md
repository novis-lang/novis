Returns the name of the cipher suite that encrypts the connection a reply came over.

A cipher suite is the set of algorithms the client and the server agree on when they connect. The
name comes from the IANA list, which most TLS tools also use: for example
`TLS_AES_128_GCM_SHA256`. TLS 1.3 and TLS 1.2 use different names. A TLS 1.2 name also says how the
keys were exchanged, such as `ECDHE_RSA`. If the list has no name for a suite, `cipher()` returns
its number in hexadecimal, which starts with `0x`.

You get a `Core\Http\TlsInfo` from `Core\Http\Response::tls()`. That method returns `null` for a
reply that did not come over TLS, so check for `null` first.

**Good to know:** the examples build their TLS connections with `Core\Test::tlsSession`, so they
run without a network. They show how to read the name, the names for each TLS version, and a check
against a list of approved ciphers.
