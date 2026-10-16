Produces a secret that you and another program share, from your own key pair and the public key that
program sent you.

Both sides do this. You use your own private key with their public key, and they use their private
key with your public key. Both sides end up with the same secret, and the secret is never sent over
the network. Someone who reads both public keys on the way still cannot work out the secret.

The result is not ready to be used as a key. Give it to `Core\Crypto::expandKey` together with a
short text that names what the key is for, such as `"chat v1"`. That gives you a key for
`Core\Crypto::seal`.

**Good to know:** both keys must be of the same kind, `P256` or `X25519`. An `Ed25519` key signs and
agrees nothing, and an RSA key cannot agree at all.
