Makes a `Core\Jwe\Key` from a secret key of 32 bytes. Use it when the program that encrypts and the
program that decrypts both have the same secret key, for example one server, or two services that
share one configuration.

`Core\Crypto::generateKey` returns a key of the right length. A key with another length throws a
`LogicError`. The error message says the length of the key but never shows the key itself, so the
message is safe to log.

A token made with this key uses the JWE algorithm `dir`: the secret key encrypts the text directly.
This is the fastest key for `Core\Jwe::encrypt` and `Core\Jwe::decrypt`.

**The examples below** make a new key, read a key that is saved as text, and share one key between
two services.
