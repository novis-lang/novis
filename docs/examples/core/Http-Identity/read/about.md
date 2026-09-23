Reads your client certificate and its private key, and checks that they belong together.

Some servers, often banks and payment services, ask the client to prove who it is when the
connection starts. For this you get two files: a certificate chain in PEM format, and a private key.
Read the key with `Core\Crypto\KeyPair::read` first, then pass both here. The result is a
`Core\Http\Identity`. You give it to a request as the `identity` option, and it is sent only when
the server asks for it.

The first certificate in the chain must be yours, and the key must match it. The certificates after
it are only passed on to the server. If the key belongs to a different certificate, or the text is
not a certificate at all, this throws a `LogicError`. So you find a wrong file when the program
starts, and not later when a request fails.

**Good to know:** an `X25519` key cannot sign anything, so it is not allowed here. Use a `P256`,
`Ed25519` or RSA key.
