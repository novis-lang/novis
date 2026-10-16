Makes a `Core\Jwe\Key` from the public key of another person or service. Use it when you want to
send them a message that only they can read. `Core\Jwe::encrypt` encrypts the message with this key.
The key cannot decrypt anything. Only the matching key pair can do that, with `Core\Jwe\Key::own`.

The public key must be a `P256` or an `X25519` key. An `Ed25519` key or an RSA key throws a
`LogicError`, because these keys cannot make a shared secret (a secret value that both sides compute
from their own keys).

A token made with this key uses the JWE algorithm `ECDH-ES`. A public key does not need to be
secret, so the other side can publish it or send it over the network.

**The examples below** encrypt a message for a key pair, show that this key cannot decrypt, and send
a card number to a payment service that published its public key as JSON.
