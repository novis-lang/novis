Reports which kind of key this is, as a case of `Core\Crypto\KeyKind`.

The answer is the kind that was named when the key was read, and it never changes afterwards. That
matters most for RSA. The octets of an RSA key are the same whether the key is meant for `RsaPkcs1`
or for `RsaPss`, so nothing in the material could tell you, and the program that was handed the key
is the only one that can say.

Use it to decide what a key is for before you use it. An `X25519` key agrees a shared secret with
`Core\Crypto::agree`. The other four kinds check signatures with `Core\Crypto::verify`.

**Good to know:** the kind is kept beside the key, so asking costs nothing. A key that came from
`Core\Crypto\KeyPair::publicKey` reports the kind of the pair it belongs to.
