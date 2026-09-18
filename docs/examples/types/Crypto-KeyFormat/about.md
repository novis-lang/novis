Names which encoding a public key is read out of, or written back into.

A public key is a number, and there are several settled ways of writing one down. `Core\Crypto\KeyFormat`
has one case per encoding a browser's WebCrypto exports, under the same names it uses: `Raw` is the key
material with nothing around it, `Spki` is the DER document that carries the key under the name of the
algorithm it belongs to, and `Jwk` is the JSON form an identity provider publishes. Reading a key names
the encoding it arrived in; writing one names the encoding the other end asked for.

**Good to know:** not every kind of key has every encoding. `Raw` is only defined where the key is a
point or a string of octets, so asking for it on an RSA key is a mistake in the program rather than a
key that failed to parse. And a key written back out is written from the key, not from the octets it
arrived in — so one key has one spelling per encoding, however it reached you. The examples write one
key three ways, carry it in on one encoding and out on another, and publish a set of keys as JSON.
