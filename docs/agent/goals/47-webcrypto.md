---
milestone: M8
---
# Loop goal 47 — a browser and a Novis server read each other's encrypted data

Anything a browser encrypts with WebCrypto, a Novis server can decrypt, and the other way round, with no
JavaScript crypto library on the browser side. **One streamlined API serves every cipher**:
`Core\Crypto::seal` and `open` take a closed, **required** `Cipher` enum — XChaCha20-Poly1305 or
AES-256-GCM — the way `Core\Hash::of` takes a required `Core\Digest`. Beside them sit the members whose
parameters genuinely differ: PBKDF2 and HKDF derivation, and ECDH over P-256 and X25519 through typed key
objects. The protocol roster gains **`Core\Jwe`**, a closed subset of JSON Web Encryption (RFC 7516/7518)
whose algorithm follows from the type of the key it is handed. What proves it is Node's own
`crypto.subtle`, the W3C Web Cryptography API browsers expose, round-tripping every mode against the built
`nvs` binary.

Nothing is taken away: XChaCha20-Poly1305 stays and every `Core\Digest` case stays. Nothing has shipped
publicly, so `seal` and `open` change signature, and **every existing call site moves in the same commit**
— no existing case, example or floor check is left red.

## Why here

Directly after goal `template-format`, on the user's call rather than a dependency: it shares no file with
that goal (`nvs-lsp`, `editors/vscode`) or with the others near it, so its position costs nothing and
moves nothing. It opens `nvs-stdlib` and the workspace manifest and nothing else.

Before goal `gap-zero`, for that goal's standing reason — a register is emptied after everything that
adds to it has run, and this goal adds and closes rows.

What it needs already built, all on disk: `Core\Crypto`'s one construction and its three seams
(`crates/nvs-stdlib/src/crypto.rs:103-113`), every draw through `crate::random::draw`
(`crates/nvs-stdlib/src/crypto.rs:115-121`), `Core\Hash::of`'s required-enum row
(`crates/nvs-stdlib/src/hash.rs:304-312`), JWT's compare-only header reading that JWE copies
(`crates/nvs-stdlib/src/jwt.rs:11-22`), `Core\Signature`'s key ring and `tainted` payload answer
(`docs/spec/01-core-library.md:1175`), base64url in `crates/nvs-stdlib/src/encoding.rs`, and the
RustCrypto base the lockfile already carries — `aead` 0.6, `cipher` 0.5, `hmac` 0.13, `sha2` 0.11, `pbkdf2`
0.13 and `curve25519-dalek` 5.

## Stage 0 — the catch-up

Sentences on disk this goal makes wrong. Each is corrected in the stage that makes it wrong. Re-grep
before editing: these are anchors, and files move.

- `crates/nvs-stdlib/src/crypto.rs:1-2` — "as three members that take a key and a message and nothing
  else". Stage 4.
- `crates/nvs-stdlib/src/crypto.rs:8-19` — § *No cipher argument*. Its argument was against a cipher
  named as a string and against a mode that is not authenticated; a closed enum of AEADs is neither, and
  `crates/nvs-stdlib/src/jwt.rs:24-28` is the argument for why a program choosing its own algorithm
  chooses nothing an attacker supplied. Stage 4 rewrites the section as a whole to say that.
- `crates/nvs-stdlib/src/crypto.rs:36-40` — "AES-GCM was the alternative and loses on two counts". Still
  why XChaCha is the construction to *prefer* when both ends are Novis; stage 4 rewrites it to say so,
  and that AES-GCM is here for interop.
- `crates/nvs-stdlib/src/crypto.rs:103-113` — "there is exactly one `XChaCha20Poly1305::new_from_slice`".
  Stage 4 says what the second construction is and that it has one home too.
- `docs/reference/core/Crypto.md:2-11` — the summary and opening say "no cipher, mode, padding or nonce
  argument" and "three members and no cipher name anywhere in them". Stage 4.
- `docs/rules/security/protocol-roster.md:1-3` and `docs/spec/01-core-library.md:1175` — a roster of five.
  JWE is the sixth. Stage 2.
- `docs/rules/security/protocol-roster.md:14-16` — "`Core\Signature` is not: no member, no module, no
  case". Already wrong: `crates/nvs-stdlib/src/signature.rs` exists. Stage 2 corrects it while it is in
  the file.
- `docs/spec/02-php-migration.md:811-812` — `hash_hkdf` is a `member` row naming no member, and
  `hash_pbkdf2` sends "a key rather than a password" to a `Core\Crypto` member that does not exist.
  Stage 4, once the members are registered.

## Stage 1 — the floor

Goal `template-format`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.
It holds `examples/crypto.nvs` as an `exact` check (`docs/agent/loop-goal.toml:2691-2692`), so stage 4
changes that file's source and never its six lines of output.

## Stage 2 — the record

One new record, and no other number. It creates `core-classes/crypto-interop-tier` and
`security/jwe-compact-subset`, both `designed`, and modifies `security/protocol-roster`. Its body is
§ *Standing decisions* below, argued: this is transcription, not design. It fixes the spellings the
surface below leaves open — the enums' namespace, under `Core\Digest`'s precedent, and any name
`rule:core-api/verb-lexicon` refuses — and the numbers it tunes stay inside the bounds given there.

The spec rows move with it: `docs/spec/01-core-library.md:1173` becomes the streamlined surface, and a
`Core\Jwe` row joins § 16. The migration rows wait for stage 4 (the handoff says why).

## Stage 3 — the keystone: the primitives, under their published vectors

The dependencies, then one crate-private function per primitive in `crates/nvs-stdlib/src/crypto.rs` (or
a `crypto/` directory beside it, if the module doc says why), each proven against the vectors its
specification publishes before any `Core` member calls it:

| Primitive | Vectors |
|---|---|
| AES-256-GCM | NIST's GCM test vectors (the 256-bit key cases), plus a flipped tag refused |
| PBKDF2-HMAC-SHA256 | RFC 7914 § 11 |
| HKDF-SHA256 | RFC 5869 Appendix A.1-A.3 |
| X25519 | RFC 7748 § 5.2 and § 6.1, plus a low-order point refused |
| ECDH P-256 | RFC 5903 § 8.1, plus a point off the curve refused |
| AES Key Wrap, internal to PBES2 | RFC 3394 § 4.6 |
| Concat KDF, internal to ECDH-ES | RFC 7518 Appendix C |

The vectors go in as test source with the section they came from named beside each one. Every key, nonce
and salt a primitive draws comes from `crate::random::draw`, so a `#[Test(seed: …)]` reproduces it.

## Stage 4 — the `Core\Crypto` surface, and every call site with it

The surface § *Standing decisions* fixes, five edits per member (conventions.md § *A `Core` member*).
`seal` and `open` gain their required `Cipher`, and **in the same commit every existing call moves to
it**, so the tree never holds a red case:

| File | What changes |
|---|---|
| `crates/nvs-stdlib/src/crypto.rs:158-195` | the `seal` and `open` rows gain `CoreTy::Enum` for the cipher, with `defaults: &[]` |
| `examples/crypto.nvs:53-66` | the three calls pass `Cipher::XChaCha20Poly1305`; the frozen output does not change |
| `tests/conformance/core/crypto-seals-and-opens-with-no-cipher-argument.nvst` | renamed `crypto-seals-and-opens-under-the-cipher-the-call-names.nvst` (no floor names it by path), and its calls take the cipher |
| the other five `tests/conformance/core/crypto-*.nvst` | every `seal`/`open` call takes the cipher; expected output unchanged |
| `docs/reference/core/Crypto.md` | rewritten for the new surface; `python tools/reference.py` regenerates `docs/novis.md` from it |
| `crates/nvs-stdlib/src/signed_cookie.rs` | nothing: it reaches the construction through the crate-private seams and stays on XChaCha |

`generateKey()` does not change, so the ~40 JWT, CSRF, `Core\Signature`, signed-cookie, `Core\Uri` and
router cases that draw a key through it are untouched. A new reject case proves the old two-argument
call no longer compiles. Then `docs/spec/02-php-migration.md:811-812` become two `member` rows naming
`deriveKey` and `expandKey`. Stage 0's `crypto.rs` and `Crypto.md` sentences are rewritten here.

## Stage 5 — `Core\Jwe`

A new `crates/nvs-stdlib/src/jwe.rs`, registered in `crates/nvs-stdlib/src/registry.rs` beside
`crate::jwt::CLASS` (`:1706`): compact serialization, `encrypt` and `decrypt`, the subset the standing
decisions fix, over stage 4's crate-private functions — never a second copy of a primitive.

## Stage 6 — the round trip with WebCrypto itself

`tools/webcrypto-interop.mjs`: a Node script using only `globalThis.crypto.subtle`, with no npm package
and no network. For every mode it encrypts with WebCrypto and decrypts with the built `nvs` binary, then
the other way round, and prints one line per direction. Node 24's WebCrypto does X25519 and P-256
(checked while this goal was written). Then `examples/webcrypto.nvs`, the example
`rule:testing/four-proofs` asks for, frozen by an `exact` check.

## Stage 7 — the rulebook

Flip `core-classes/crypto-interop-tier` and `security/jwe-compact-subset` to `shipped`, with `guardedBy`
filled from this goal's cases and tests, and `python tools/rules.py --render`.

## Standing decisions

- **One member per job, and a required enum where the parameters are the same.** Where two algorithms
  take the same parameters they are one member with a closed enum argument, as `Core\Hash::of` takes a
  `Core\Digest`. Where the parameters differ they are separate members. No member name carries an
  algorithm, and no algorithm is ever a string.
- **No algorithm argument has a default — not a cipher, not a digest, not a curve.** A call names what it
  uses. `Core\Hash` already holds this: every row is `defaults: &[]`
  (`crates/nvs-stdlib/src/hash.rs:304-343`), and it is not touched. A member that answers a key with no
  algorithm in it takes no algorithm argument at all, which is `generateKey`: its 32 octets are the one
  key length both ciphers and every roster protocol share, and one ring of them is handed to
  `Core\Signature` and `Core\SignedCookie` alike (`crates/nvs-stdlib/src/signature.rs:48-73`).
- **The surface.** Spellings may be adjusted by the record; the shape may not:

  | Member | Does |
  |---|---|
  | `Crypto::generateKey(): secret bytes` | 32 random octets, a key for either cipher and for every roster protocol |
  | `Crypto::seal(bytes $message, secret bytes $key, Crypto\Cipher $cipher): bytes` | `nonce ‖ ciphertext ‖ tag` |
  | `Crypto::open(bytes $sealed, secret bytes $key, Crypto\Cipher $cipher): bytes` | the plaintext, or one forgery `RuntimeError` |
  | `Crypto::deriveKey(secret string $password, bytes $salt, uint $iterations): secret bytes` | PBKDF2-HMAC-SHA256, 32 octets |
  | `Crypto::expandKey(secret bytes $material, bytes $salt, string $info): secret bytes` | HKDF-SHA256, 32 octets |
  | `Crypto::generateKeyPair(Crypto\Curve $curve): Crypto\KeyPair` | P-256 or X25519 |
  | `Crypto::agree(Crypto\KeyPair $mine, Crypto\PublicKey $theirs): secret bytes` | the raw shared secret, meant for `expandKey` |
  | `Crypto\PublicKey::read(bytes $encoded, Crypto\Curve $curve, Crypto\KeyFormat $format): Crypto\PublicKey` | raw, SPKI or JWK, validated on read |
  | `$publicKey->write(Crypto\KeyFormat $format): bytes` | the same three forms |
  | `$publicKey->curve(): Crypto\Curve` | which curve it is on |
  | `Crypto\KeyPair::read(secret bytes $pkcs8, Crypto\Curve $curve): Crypto\KeyPair` | a stored pair back |
  | `$keyPair->write(): secret bytes` | PKCS#8, so a server keeps its pair across requests |
  | `$keyPair->publicKey(): Crypto\PublicKey` | the half that is sent |
  | `Jwe::encrypt(string $payload, secret bytes\|Crypto\PublicKey\|secret string $key): string` | the compact token |
  | `Jwe::decrypt(string $token, array<secret bytes>\|array<Crypto\KeyPair>\|secret string $keys): tainted string` | the payload, or one `RuntimeError` |

  `Crypto\Cipher` is `XChaCha20Poly1305` and `Aes256Gcm`; `Crypto\Curve` is `P256` and `X25519`;
  `Crypto\KeyFormat` is `Raw`, `Spki` and `Jwk`. Opening under the wrong cipher is a forgery, never
  plausible bytes. XChaCha20-Poly1305 is the one to prefer when both ends are Novis
  (`crates/nvs-stdlib/src/crypto.rs:21-40`), and `seal`'s doc says so — as advice, not as a default.
- **One fallback, because one thing was not checked when this was written.** If a member row cannot
  declare a union with `secret` members, JWE's key becomes a `Jwe\Key` built by one static per kind
  (`shared`, `password`, `recipient`, `own`), with still two `Jwe` members. The record says whether it
  held.
- **Existing call sites move with the signature, in the same commit.** Nothing has shipped publicly, so
  `seal` and `open` change signature; stage 4's table is every file that calls them, and a session that
  finds one more adds it rather than leaving it red. A case's source is corrected, never its expected
  output, and a case is renamed only where its name has become false and no floor names it by path.
  What stays: XChaCha20-Poly1305 and its sealed layout, `generateKey()`, every `Core\Digest` case and
  every `Core\Hash` signature, `Core\Password`, and `signed_cookie.rs`'s use of the construction.
- **The roster, closed.** In: AES-256-GCM, PBKDF2-HMAC-SHA256, HKDF-SHA256, ECDH over P-256 and X25519.
  Internal only, never a member: AES Key Wrap (PBES2 needs it) and Concat KDF (ECDH-ES needs it). Out,
  and not to be added by any session of this goal: AES-CBC, AES-CTR, AES-128, RSA in any form, and JWE's
  `A*CBC-HS*` content encryption. ECDSA and Ed25519 signatures are signing, not this goal.
- **AES-GCM's sealed bytes are `nonce(12) ‖ ciphertext ‖ tag(16)`** — exactly what WebCrypto's `encrypt`
  answers with its IV put in front, so a browser splits at byte 12 and does nothing else. XChaCha's stay
  `nonce(24) ‖ ciphertext ‖ tag(16)`. The nonce is random and drawn through `crate::random::draw`, with no
  nonce parameter; the record and `seal`'s doc state AES-GCM's 2^32-messages-per-key bound and name
  XChaCha as the answer when that bound matters. No additional-data parameter in this goal; JWE uses AAD
  internally.
- **Keys.** A symmetric key is `secret bytes`, length-checked: the wrong length is a `LogicError` naming
  the length wanted, never the bytes got (`crates/nvs-stdlib/src/crypto.rs:68-74`). A private key never
  leaves a `KeyPair` except as `secret bytes` through `write`. **Every public key is validated at
  `read`**: on the curve for P-256, and an all-zero X25519 shared secret is refused at `agree`. A public
  key off the wire that fails is a `RuntimeError` (a verdict); a malformed key the program built is a
  `LogicError` (a bug). Two keys on different curves given to `agree` is a `LogicError`. JWK export of a
  private key is out of scope.
- **PBKDF2 has a floor and a ceiling, and both are checked before the first HMAC.** The iteration count
  is a required parameter, because the other end chose it. It is refused below 100,000 and above
  2,000,000, and a salt shorter than 16 octets is refused. The record may move either number with a
  reason, never remove one. The ceiling is what makes PBES2 safe: `p2c` in a JWE header is
  attacker-supplied, and without it one token buys unbounded CPU.
- **JWE, the subset.** Compact serialization only. `enc` is `A256GCM` and nothing else. **The algorithm
  comes from the key's type** (`rule:security/algorithm-comes-from-the-key`): `secret bytes` means `dir`,
  a `PublicKey` or `KeyPair` means `ECDH-ES` (direct agreement, no key wrap), and a `secret string`
  password means `PBES2-HS256+A128KW`. The header's `alg` and `enc` are read only to be **compared**, as
  `crates/nvs-stdlib/src/jwt.rs:11-22` does, and a mismatch is a refusal. The allowed header parameters
  are `alg`, `enc`, `epk`, `p2s`, `p2c`, `kid`, `typ` and `cty`. `zip`, `crit`, `jku`, `x5u`, `x5c`, a
  `jwk` other than `epk`, and anything unknown are refused, because `zip` is a decompression bomb and the
  URL-bearing ones are fetches. The protected header is length-capped before it is parsed.
- **JWE answers like the rest of the roster.** The payload is `string` in and **`tainted string`** out,
  as with `Core\Signature::verify`, and every failure of authenticity is one `RuntimeError` with one
  sentence. Decrypt's key ring is tried in order, as `Core\Signature::verify`'s is, except that a
  password is exactly one, because every try costs a full derivation.
- **Dependencies, all pure Rust, all on the generation already locked**: `aes-gcm` 0.11 (brings `aes`
  0.9), `aes-kw` 0.3, `p256` 0.14 with its `ecdh` feature, `x25519-dalek` 3.0, `hkdf` 0.13, and `pbkdf2`
  0.13 as a direct dependency. Default features are off and no `getrandom` anywhere: randomness reaches
  them through an adapter over `crate::random::draw`. **`p256`'s curve arithmetic has never been
  independently audited** (its README says so); the record names that and accepts it, because P-256 is
  the one curve every browser's WebCrypto has and the crate is written constant-time. Never a C crypto
  crate: `deny.toml:70-73` bans OpenSSL, and `ring` and `aws-lc-rs` are C too. Each added crate
  regenerates `THIRD-PARTY-LICENSES.txt` in the same commit
  (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`). If a pinned version will
  not build against the locked base, take the nearest one that does and say so in the commit.
- **The interop proof is WebCrypto itself, not a JS library.** Node's `crypto.subtle` is the W3C API
  browsers ship, so the script needs no package and no network. If a mode fails in the script, the Novis
  side is presumed wrong until a published vector says otherwise.
- **What it spends**: per call, a few hundred bytes of cipher or curve state on the stack and one output
  buffer charged to the request through `nvs_runtime::budget`, passed through `nvs_runtime::affordable`
  first, as `seal` is today. A `KeyPair` holds its private scalar for as long as the program holds the
  object. PBKDF2 is bounded in CPU by its ceiling. Nothing is held between calls, so it is O(in-flight).
- **Not this goal**: JWS with ES256 or EdDSA, JWK export of private keys, an additional-data parameter,
  streaming encryption. A session that finds one on its path writes it to the handoff's `## Backlog`.
