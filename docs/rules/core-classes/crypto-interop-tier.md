`Core\Crypto`'s algorithm roster is closed, every algorithm argument is required, and no member name
carries an algorithm.

The roster is AES-256-GCM beside XChaCha20-Poly1305, PBKDF2-HMAC-SHA256, HKDF-SHA256, ECDH over P-256
and X25519, and four signature algorithms — RSASSA-PKCS1-v1_5 and RSASSA-PSS over SHA-256, ECDSA over
P-256, and Ed25519. AES Key Wrap and the Concat KDF are internal to PBES2 and ECDH-ES and never members.
Out, permanently: AES-CBC, AES-CTR, AES-128, RSA encryption in any form, RSA key generation, P-384,
ES384, RS384, RS512, PS384, PS512, ES256K, and JWE's `A*CBC-HS*` content encryption. A closed roster that
is only a habit reopens at the first ticket, so the list is written down.

**Where two algorithms take the same parameters they are one member with a closed enum argument**, the
way `Core\Hash::of` takes a `Core\Digest`; where the parameters differ they are separate members. **No
algorithm argument has a default** — not a cipher, not a digest, not a curve. A default is how a call
site copied from another file keeps an algorithm nobody re-read, and making the argument required buys a
`grep` that finds every use of a primitive on the day it has to be retired. The one member answering a
key with no algorithm in it, `generateKey()`, takes no algorithm argument at all: its 32 octets are the
single length both ciphers and every roster protocol share.

**AES-256-GCM is on the roster for interoperability and its sealed bytes say so**: `nonce(12) ‖
ciphertext ‖ tag(16)`, which is exactly what WebCrypto's `encrypt` answers with its IV put in front, so
a browser splits at byte 12 and does nothing else. XChaCha's stay `nonce(24) ‖ ciphertext ‖ tag(16)` and
remain the pair to prefer when both ends are Novis — advice on the member's reference card, never a
default. The nonce is drawn by the member and there is no nonce parameter, so AES-GCM carries its
birthday bound around 2^32 messages under one key; that bound is the price of reading what a browser
wrote, and the answer when a program approaches it is the other cipher rather than a counter the caller
keeps.

**A key is validated where it is read, not where it is used.** Every public key is checked at `read` — on
the curve for P-256, inside 2048–8192 bits for RSA — and an all-zero X25519 shared secret is refused at
`agree`, so there is no later site at which the check could be forgotten. An RSA key's scheme is fixed
when it is read, which is `rule:security/algorithm-comes-from-the-key` held for the one key type two JWS
algorithms share, and an RSA pair is never generated. A key off the wire that fails is a `RuntimeError`;
a malformed key the program built is a `LogicError`.

The interop claim is checkable rather than intended: `bun nv webcrypto-vectors` freezes WebCrypto's
own output for every algorithm here, derived from labels so a rerun writes the same bytes.
