Draws a new private key of the kind you name, and gives you the pair it belongs to.

A key pair is two halves that belong together. The private half stays inside your program. It signs
messages with `Core\Crypto::sign`, and it agrees a shared secret with another program through
`Core\Crypto::agree`. The public half is the one you publish or send to somebody else, so they can
check your signatures or agree that same secret. `Core\Crypto\KeyPair::publicKey` gives you that
half.

There is no key size to choose, because each kind has one size. `P256` agrees and signs, `X25519`
only agrees, and `Ed25519` only signs.

Every call draws a new pair. To keep one pair across restarts, store the bytes that
`Core\Crypto\KeyPair::write` gives you and read them back with `Core\Crypto\KeyPair::read`.

**Good to know:** the two RSA kinds are not drawn here. An RSA key is issued to you by somebody
else, so you read that file with `Core\Crypto\KeyPair::read`.
