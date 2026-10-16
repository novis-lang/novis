Signs a message with the private half of a key pair, so that anybody holding the public half can
check who wrote it and that it arrived unchanged.

There is no algorithm to choose. The scheme is the kind of the key pair: `Ed25519` signs as itself,
`P256` signs as ECDSA with SHA-256, and the two RSA kinds sign over SHA-256. A call cannot name a
scheme the key is not for.

A signature is bytes that travel next to the message, often as text in a header or a field. The
other side calls `Core\Crypto::verify` with the message, the signature and your public key.

Two signatures over one message are not always equal. `Ed25519` and `RsaPkcs1` give the same
signature every time. `P256` and `RsaPss` use fresh random data in every call, so two signatures
differ and both are valid.

**Good to know:** an `X25519` pair signs nothing, and this member throws an error for one. That kind
is for `Core\Crypto::agree`.
