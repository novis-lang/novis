Returns the certificates that the server of a TLS connection sent, as an array of texts.

A server proves who it is with a certificate. It usually sends more than one: its own certificate
first, and then the certificate of the authority that signed it. `peerChain()` returns them in
that order. Each entry is one certificate in PEM form (a text that starts with
`-----BEGIN CERTIFICATE-----`), which other tools can read. The texts are tainted (they came from
outside the program), because the other server chose them. The array is empty if the server sent
no certificate.

You get a `Core\Http\TlsInfo` from `Core\Http\Response::tls()`. That method returns `null` for a
reply that did not come over TLS, so check for `null` first.

**Good to know:** the examples build their TLS connections with `Core\Test::tlsSession`, so they
run without a network. They read the certificates, join them into the text of one `.pem` file, and
notice when a server starts to use a different certificate.
