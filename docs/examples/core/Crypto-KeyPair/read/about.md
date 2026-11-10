Reads a private key your program already has, and gives you the key pair it belongs to.

The key must be PKCS#8, so a key from another program and a key from `Core\Crypto\KeyPair::write`
are read the same way. It may be binary DER, or one PEM block that starts with
`-----BEGIN PRIVATE KEY-----`.

You always name the kind of key. The file does not say whether an RSA key is for `RsaPkcs1` or for
`RsaPss`. If somebody swapped the file for a key of another kind, `read` throws an error.

The key is parsed once, here. The pair can then sign with `Core\Crypto::sign` and agree a shared
secret with `Core\Crypto::agree`. `Core\Crypto\KeyPair::publicKey` gives the half you send to other
people.

**Good to know:** a key in another format throws a `LogicError`. Examples are a key with a password
(`ENCRYPTED PRIVATE KEY`), an RSA key in PKCS#1 (`RSA PRIVATE KEY`) and an EC key in SEC1
(`EC PRIVATE KEY`). The message names the format and the command that converts the file to PKCS#8.
