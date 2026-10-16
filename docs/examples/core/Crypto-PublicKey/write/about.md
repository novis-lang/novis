Writes a public key out as octets, in the encoding you name, so you can send it or store it.

The three encodings are the ones `Core\Crypto\PublicKey::read` accepts: the plain key material
(`Raw`), a `SubjectPublicKeyInfo` document (`Spki`), and a JSON Web Key (`Jwk`). A browser reads all
three, so a key crosses to one in whichever form the other end asked for.

The octets are written from the key itself, not copied out of the octets the key arrived in. One key
therefore has one spelling per encoding, however it was read. A `Jwk` holds the members the standard
requires, sorted by name, and nothing else. Members such as `ext` and `key_ops` are left out, even
when the key was read from a document that had them.

**Good to know:** an RSA key has no `Raw` form, so asking for one is an error in your program.
`Spki` and `Jwk` are the encodings it has.
