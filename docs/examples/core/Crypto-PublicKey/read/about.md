Reads a public key that somebody else sent you, and gives you the key as an object.

A public key arrives as octets: the plain key material (`Raw`), a `SubjectPublicKeyInfo` document
(`Spki`), or a small JSON document (`Jwk`). These are the three encodings a browser's WebCrypto
writes, so you name the one the octets are in. You also name the kind of key, because the octets do
not always say it. An RSA key looks the same whether it is meant for `RsaPkcs1` or for `RsaPss`.

The key is checked here, once. A point that is not on the curve, an RSA key that is too small or too
large, and a document that does not parse are all refused at the read. Every later member is then
handed a key that has been checked.

**Good to know:** the key you get back checks signatures with `Core\Crypto::verify` and agrees a
shared secret with `Core\Crypto::agree`.
