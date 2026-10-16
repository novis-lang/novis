Returns the name in the certificate of the server a connection went to, as text like
`CN=api.example.com, O=Shop`.

A certificate names who it belongs to. The name is a list of `KEY=value` pairs separated by `, `.
`CN` is the server name, `O` is the organization, and `C` is the country. `subject()` returns the
whole list as one string. The text is tainted (it came from outside the program), because the
other server chose what is written in its certificate.

You get a `Core\Http\TlsInfo` from `Core\Http\Response::tls()`. That method returns `null` for a
reply that did not come over TLS, so check for `null` first.

**Good to know:** the examples build their TLS connections with `Core\Test::tlsSession`, so they
run without a network. They show the whole subject, how to take the server name out of it, and a
check that a payment partner's certificate names the expected organization.
