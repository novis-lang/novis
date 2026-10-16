Reads a private key your program already has, and gives you the key pair it belongs to.

A private key is stored as PKCS#8. That is a standard format, so a key written by another program
and a key from `Core\Crypto\KeyPair::write` are read the same way. The octets may be the binary DER
form, or one PEM block that starts with `-----BEGIN PRIVATE KEY-----`.

You always name the kind of key. The file does not say whether an RSA key is for `RsaPkcs1` or for
`RsaPss`. Naming the kind also means a file that was swapped for a key of another kind is refused
here.

The key is parsed once, at the read. A pair can then sign with `Core\Crypto::sign` and agree a
shared secret with `Core\Crypto::agree`. `Core\Crypto\KeyPair::publicKey` gives you the half you
send to other people.

**Good to know:** a key with a password and an old PKCS#1 key are both refused. Their PEM blocks say
`ENCRYPTED PRIVATE KEY` and `RSA PRIVATE KEY`. Convert such a file to PKCS#8 first.
