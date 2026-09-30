Says which encoding a public key is read from or written to.

A public key can be written as bytes in several standard ways. `Core\Crypto\KeyFormat` has one case
for each encoding that a browser's WebCrypto API exports, and the cases have the same names. `Raw`
is the key bytes alone. `Spki` is a DER document that contains the key and the name of its
algorithm. `Jwk` is the JSON form that identity providers publish. When you read a key, you pass
the encoding the bytes are in. When you write a key, you pass the encoding the receiver needs.

**Good to know:** `Raw` does not exist for RSA keys, and reading an RSA key as `Raw` throws a
`LogicError`. `Spki` works with every kind of key. Writing a key always gives the same bytes for
the same key and encoding. It does not matter which encoding the key was read from.

**The examples below** write one key in all three encodings, read a key in one encoding and write
it in another, and publish a list of keys as JSON.
