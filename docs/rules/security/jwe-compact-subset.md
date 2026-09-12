`Core\Jwe` reads and writes compact JWE with `A256GCM` alone, and the static that built the key is what
picks the key-management algorithm.

`rule:security/algorithm-comes-from-the-key` applied to encryption: `Jwe\Key::shared` means `dir`,
`Jwe\Key::password` means `PBES2-HS256+A128KW`, and `Jwe\Key::recipient` and `Jwe\Key::own` mean
`ECDH-ES` — direct agreement, no key wrap, empty `apu` and `apv`. The header's `alg` and `enc` are read
**only to be compared**, and a mismatch is a refusal. A named constructor rather than a union because a
shared key and a password are both confidential octets and there is no type that tells them apart; what
the constructor buys back is that the algorithm now comes from a name the caller wrote rather than from
an inference the caller cannot see.

**The allowed protected-header parameters are `alg`, `enc`, `epk`, `p2s`, `p2c`, `kid`, `typ` and
`cty`, and everything else is refused** — `zip` because it is a decompression bomb
(`rule:core-classes/decompression-bound`), `jku`, `x5u` and `x5c` because they are fetches the token is
talking the program into, `crit` and a `jwk` other than `epk` because they ask the verifier to act on
what the token brought with it. An unknown member in a ciphertext header is either an extension we do
not implement or an attack, with no third reading. The protected header is length-capped before it is
parsed.

**`p2s` and `p2c` are held to the member's own bounds**: PBKDF2's iteration count is refused below
100,000 and above 2,000,000 and its salt below 16 octets, checked before the first HMAC. `p2c` is
attacker-supplied, so without the ceiling one token buys unbounded CPU on the request path.

**`encrypt` writes its header canonically** — members sorted, no whitespace, at every level — which is
what lets it be held to the frozen vector set byte for byte rather than only round-tripped against
itself.

The payload is `string` in and **`tainted string`** out, as `Core\Signature::verify`'s is
(`rule:security/verification-does-not-launder`): decrypting proves who wrote it, never that it is safe.
Every refusal is one `RuntimeError` with one sentence. Decrypt's key ring is tried in order, except that
a ring holding a password key holds exactly one key, because every try costs a full derivation and an
attacker choosing the ring's length is the iteration ceiling defeated one layer up.
