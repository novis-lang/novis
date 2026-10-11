`Core\Crypto`'s algorithm roster is closed, every algorithm argument is required, and no member name
carries an algorithm.

The roster is AES-256-GCM beside XChaCha20-Poly1305, PBKDF2-HMAC-SHA256, HKDF-SHA256, ECDH over P-256
and X25519, and four signature algorithms — RSASSA-PKCS1-v1_5 and RSASSA-PSS over SHA-256, ECDSA over
P-256, and Ed25519. AES Key Wrap and the Concat KDF are internal to PBES2 and ECDH-ES and never members.
Out, permanently: AES-CBC, AES-CTR, AES-128, RSA encryption in any form, RSA key generation, P-384,
ES384, RS384, RS512, PS384, PS512, ES256K, and JWE's `A*CBC-HS*` content encryption. A closed roster that
is only a habit reopens at the first ticket, so the list is written down. Unauthenticated and legacy
modes are out by `rule:core-api/tier-roster`; the size variants are out because two of everything
doubles the surface for no property a program can name; RSA encryption is out because RSA1_5 is a
padding oracle and RSA-OAEP buys nothing ECDH-ES does not, at a fraction of the key size.

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
keeps, which is a worse failure with a wider blast radius. Opening under the wrong cipher is a forgery,
never plausible bytes: both constructions authenticate before they decrypt, so a mismatched
`Crypto\Cipher` gets the same one `RuntimeError` sentence as every other message that is not authentic.

**A key is validated where it is read, not where it is used.** Every public key is checked at `read` — on
the curve for P-256, inside 2048–8192 bits for RSA — and an all-zero X25519 shared secret is refused at
`agree`, so there is no later site at which the check could be forgotten. An RSA key's scheme is fixed
when it is read, which is `rule:security/algorithm-comes-from-the-key` held for the one key type two JWS
algorithms share. An RSA pair is never generated: `generateKeyPair` refuses both RSA kinds with a
`LogicError` naming `Crypto\KeyPair::read`, because a program is handed its RSA key by whoever issued
it. A key off the wire that fails is a `RuntimeError`; a malformed key the program built is a
`LogicError`. A JWK `read` accepts what a browser exports, `ext` and `key_ops` included, and refuses one
carrying `d`, because a private key handed over as a public one is a bug rather than an attack. A
private key leaves a `Crypto\KeyPair` only as `secret bytes` PKCS#8, and `Crypto\KeyPair::read` takes
DER or one PEM `PRIVATE KEY` block, the form a service-account file carries, and refuses PKCS#1 and
encrypted PKCS#8.

**PBKDF2's iteration count is required**, because the other end of an already-agreed protocol chose it
and a default would be a number that is either wrong or ignored. Its floor and ceiling are frozen in the
vector set as refusals, so neither number moves without that set being regenerated
(`rule:security/jwe-compact-subset` gives the bounds and why the ceiling matters).

The interop claim is checkable rather than intended: `bun nv webcrypto-vectors` freezes WebCrypto's
own output for every algorithm here, derived from labels so a rerun writes the same bytes. A suite
written against our own implementation proves only that our encoder and our decoder agree; the other
side's output is what proves interoperability.

- **Memory:** per call, a few hundred bytes of cipher or curve state on the stack and one output buffer
  charged to the request's budget before it is allocated. A `Crypto\KeyPair` holds its private scalar for
  as long as the program holds the object. Nothing is held between calls, so all of it is O(in-flight).

**Declined:**
- **A `string` algorithm argument, or one enum spanning every primitive.** One member taking any
  algorithm is how a program ends up with `aes-256-ecb` in one file. An enum is safer than a string but
  still lets `seal` be handed a signature algorithm; separate members where the parameters differ make
  that unrepresentable.
- **Defaulting the cipher to XChaCha20-Poly1305.** A default is invisible at the call site, and the call
  sites that matter are the interoperating ones, where the cipher is not ours to choose.
- **RSA-OAEP, for an issuer that encrypts ID tokens.** It is rare and opt-in at every provider, and it
  would bring RSA encryption's whole padding surface in for one feature.
- **JWK export of a private key, an additional-data parameter, and streaming encryption.** None is on the
  surface; JWE uses additional data internally, where the protocol fixes what it is.
