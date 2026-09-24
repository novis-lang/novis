Makes a `Core\Jwe\Key` from your own key pair. `Core\Jwe::decrypt` uses this key to read messages
that other people encrypted for your public key with `Core\Jwe\Key::recipient`. Only the key pair
that belongs to that public key can read them.

The key pair must be a `P256` or an `X25519` key pair. An `Ed25519` or RSA key pair throws a
`LogicError`, because these keys cannot make a shared secret.

`Core\Jwe::encrypt` also accepts this key. The result is a message that only this same key pair can
read.

Keep the key pair secret. `$pair->write()` returns its bytes, so a server can save the key pair and
read it again with `Core\Crypto\KeyPair::read`.

**The examples below** decrypt a message sent to a key pair, show the error for a key pair that can
only sign, and show a server that saves its key pair and reads it again for each request.
