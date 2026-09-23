Returns the name of the certificate authority that signed the certificate of the server a
connection went to, as text like `CN=Example Root CA, O=Example Trust`.

A certificate authority is an organization that signs certificates. Its signature says that the
certificate really belongs to the server named in it. `issuer()` returns the authority's name in
the same `KEY=value` form as `subject()`. The text is tainted (it came from outside the program),
because the other server sent the certificate.

You get a `Core\Http\TlsInfo` from `Core\Http\Response::tls()`. That method returns `null` for a
reply that did not come over TLS, so check for `null` first.

**Good to know:** the examples build their TLS connections with `Core\Test::tlsSession`, so they
run without a network. They show the issuer, the subject and the issuer side by side, and an audit
that finds servers whose certificate a different authority signed.
