---
milestone: M8
---
# Loop goal 47 — Novis reads what a browser encrypts and what an issuer signs

Anything a browser encrypts with WebCrypto, a Novis server can decrypt, and the other way round, with no
JavaScript crypto library on the browser side. **One streamlined API serves every cipher**:
`Core\Crypto::seal` and `open` take a closed, **required** `Cipher` enum — XChaCha20-Poly1305 or
AES-256-GCM — the way `Core\Hash::of` takes a required `Core\Digest`. Beside them sit the members whose
parameters genuinely differ: PBKDF2 and HKDF derivation, and ECDH over P-256 and X25519 through typed key
objects. The protocol roster gains **`Core\Jwe`**, a closed subset of JSON Web Encryption (RFC 7516/7518)
whose algorithm follows from the type of the key it is handed. What proves it is a **frozen vector set
WebCrypto itself wrote** — `crates/nvs-stdlib/tests/vectors/webcrypto.json`, by
`tools/webcrypto-vectors.mjs` — which Novis must decrypt, and reproduce byte for byte from the same
inputs. The loop never runs Node.

**It carries JWS too, because the REST client's OAuth half needs it and only `Core` can hold it.**
`Core\Jwt` keeps HS256 for a program's own tokens and gains RS256, PS256, ES256 and EdDSA over the same
`Crypto\PublicKey` and `Crypto\KeyPair` this goal types, through `ring`, the provider TLS already links.
`Jwt::verifyIssued<T>` verifies a token another party issued against a `Jwt\KeySet`, with the issuer and
the audience required, and answers its claims as a `tainted` shape — so an ID token's list and object
claims arrive typed rather than refused. Beside JWT, `Crypto::sign` and `Crypto::verify` sign and check raw
bytes under the same key kinds — an HTTP message signature, a webhook — and `Jwt::sign` writes structured
claims under a key pair, which a signed request object needs. The same frozen set carries the JWS and raw
signature vectors WebCrypto signed.

Nothing is taken away: XChaCha20-Poly1305 stays and every `Core\Digest` case stays. Nothing has shipped
publicly, so `seal` and `open` change signature, and **every existing call site moves in the same commit**
— no existing case, example or floor check is left red.

## Why here

Directly after goal `template-format`, on the user's call rather than a dependency: it shares no file with
that goal (`nvs-lsp`, `editors/vscode`) or with the others near it, so its position costs nothing and
moves nothing. It opens `nvs-stdlib`, the workspace manifest and `nvs-types`' decode-site check, and
nothing else.

Before goal `gap-zero`, for that goal's standing reason — a register is emptied after everything that
adds to it has run, and this goal adds and closes rows.

What it needs already built, all on disk: `Core\Crypto`'s one construction and its three seams
(`crates/nvs-stdlib/src/crypto.rs:103-113`), every draw through `crate::random::draw`
(`crates/nvs-stdlib/src/crypto.rs:115-121`), `Core\Hash::of`'s required-enum row
(`crates/nvs-stdlib/src/hash.rs:304-312`), JWT's compare-only header reading that JWE copies
(`crates/nvs-stdlib/src/jwt.rs:11-22`), `Core\Signature`'s key ring and `tainted` payload answer
(`docs/spec/01-core-library.md:1175`), base64url in `crates/nvs-stdlib/src/encoding.rs`, the RustCrypto
base the lockfile already carries — `aead` 0.6, `cipher` 0.5, `hmac` 0.13, `sha2` 0.11, `pbkdf2` 0.13 and
`curve25519-dalek` 5 — and the vector set, landed ahead of the goal. For the JWS half: `ring` 0.17,
already linked as TLS's provider (`Cargo.toml:215-236`), and `Core\Request::jsonAs`'s decode-site check,
which `verifyIssued<T>` joins (`crates/nvs-types/src/expr/args.rs:1517-1528`).

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
- `crates/nvs-stdlib/src/jwt.rs:11-28` — § *The algorithm comes from the key, and there is one of it*:
  "[`ALG`] is the only algorithm this class knows". Stage 6 rewrites the section as a whole: the
  algorithm still comes from the key, and a key now has one of five kinds to come from.
- `crates/nvs-stdlib/src/jwt.rs:66-99` — § *A claim is text* names the widening stage 6 builds. Stage 6
  rewrites it to say `verify` still answers text and `verifyIssued<T>` answers a shape.
- `Cargo.toml:215-236`, `tools/gen-attribution.py:187-197` and `crates/nvs-host/src/tls.rs:93-112` —
  `ring` admitted as TLS's provider and as nothing else. Stage 3, when `nvs-stdlib` takes it directly.
- `docs/rules/security/protocol-roster.md` and `docs/spec/01-core-library.md:1175` — JWT as HS256
  alone. Stage 2.

## Stage 1 — the floor

Goal `template-format`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.
It holds `examples/crypto.nvs` as an `exact` check (`docs/agent/loop-goal.toml:2691-2692`), so stage 4
changes that file's source and never its six lines of output.

## Stage 2 — the record

One new record, and no other number. It creates `core-classes/crypto-interop-tier`,
`security/jwe-compact-subset` and `security/jws-issued-subset`, all `designed`, and modifies
`security/protocol-roster`. Its body is
§ *Standing decisions* below, argued: this is transcription, not design. It fixes the spellings the
surface below leaves open — the enums' namespace, under `Core\Digest`'s precedent, and any name
`rule:core-api/verb-lexicon` refuses — and the numbers it tunes stay inside the bounds given there.

The spec rows move with it: `docs/spec/01-core-library.md:1173` becomes the streamlined surface, and a
`Core\Jwe` row joins § 16, and the `Core\Jwt` row at `:1175` takes the asymmetric surface. The migration
rows wait for stage 4 (the handoff says why).

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
| RSASSA-PKCS1-v1_5 over SHA-256 (RS256) | RFC 7515 Appendix A.2, signed and verified |
| RSASSA-PSS over SHA-256 (PS256) | RFC 7520 § 4.2, verified — PSS draws its salt, so only verification is byte-fixed |
| ECDSA P-256 over SHA-256 (ES256) | RFC 7515 Appendix A.3, verified, plus the same signature in DER form refused |
| Ed25519 (EdDSA) | RFC 8037 Appendix A.4, signed and verified |
| JWK thumbprint | RFC 7638 § 3.1 |

The vectors go in as test source with the section they came from named beside each one. Every key, nonce
and salt a primitive draws comes from `crate::random::draw`, so a `#[Test(seed: …)]` reproduces it, and
every primitive that draws also takes its randomness as an argument at the crate-private level, because
stage 7 reproduces WebCrypto's bytes from WebCrypto's inputs.

The four signature rows go through `ring` and nothing else in the table does. An RSA key outside
2048–8192 bits, or with an exponent `ring` refuses, is refused before any signature is checked. An
Ed25519 pair is built from a 32-octet seed drawn through `crate::random::draw`, so key generation stays on
the seam. An ES256 or PS256 *signature* draws from `ring`'s own generator, because as far as was known
when this was written `ring` takes no randomness from its caller (not checked — the session confirms it,
and if it can be supplied it goes through `draw`). That is why an ES256 or PS256 token is held to
verification rather than to its bytes, here and in stage 7.

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

**The key classes take two more kinds.** `Crypto\Curve` becomes `Crypto\KeyKind` — `P256`, `X25519`,
`Ed25519`, `RsaPkcs1` and `RsaPss` — and every row that took a curve takes a kind, so JWE, key agreement
and JWT signing share one pair of key classes rather than growing a second family. Stage 3's signature
primitives are what the new kinds read into. `Crypto::sign` and `Crypto::verify` land here, over raw
bytes and the same kinds; the `Jwt` members that sign with them are stage 6's.

## Stage 5 — `Core\Jwe`

A new `crates/nvs-stdlib/src/jwe.rs`, registered in `crates/nvs-stdlib/src/registry.rs` beside
`crate::jwt::CLASS` (`:1706`): compact serialization, `encrypt` and `decrypt`, the subset the standing
decisions fix, over stage 4's crate-private functions — never a second copy of a primitive.

## Stage 6 — JWS: `Core\Jwt` over a key pair, `Jwt\KeySet`, and claims as a shape

`crates/nvs-stdlib/src/jwt.rs` gains the asymmetric half § *Standing decisions* fixes, over stage 3's
signature primitives and stage 4's key classes — never a second copy of either:

- `Jwt::sign` takes a `Crypto\KeyPair` beside a shared key, and a trailing bag for `kid`, `typ` and
  `embedKey`. Its HS256 path, its header and its payload layout do not change, so every existing `Jwt`
  case passes untouched. Under a key pair its claims may also be structured — a shape or a deriving
  class — which a signed request object (RFC 9101, with RFC 9396's `authorization_details`) needs.
- `Jwt\KeySet`, a new registered class with one static, `read`.
- `Jwt::verifyIssued<T>`, whose type argument joins `Core\Request::jsonAs` on the decode-site roster in
  `crates/nvs-types/src/expr/args.rs:1517-1528`, checked by `check_decode_sites` and
  `check_shape_decode_site` (`crates/nvs-types/src/derive.rs:803-830` and `:910`) — the one change this
  goal makes outside `nvs-stdlib`. Decoding is `Core\Json::decodeAs`'s one walk
  (`crates/nvs-stdlib/src/json.rs:82-103`), never a second decoder.

`Jwt::verify` keeps its signature and its answer. Stage 0's two `jwt.rs` sections are rewritten here.

## Stage 7 — the frozen WebCrypto set, replayed

`crates/nvs-stdlib/tests/vectors/webcrypto.json` is on disk before this goal starts. Its `about` array is
the schema. Rust tests in `nvs-stdlib` read it with `include_str!` and `serde_json`, and for every entry:

- **`aesGcm`, `pbkdf2`, `hkdf`, `aesKw`, `ecdh`** — the crate-private primitive answers the recorded
  output from the recorded input, in both directions where there are two (`open` answers the plaintext,
  `seal` given the nonce answers the sealed bytes). Every `ecdh` key is read in all three forms —
  `raw`, `spki`, and `jwk` exactly as a browser exported it — and each agrees the same `secret`;
  `write(KeyFormat::Jwk)` answers `jwkMinimal`.
- **every `refusals` entry** — refused by the member that owns the rule: AES-GCM with the one forgery
  `RuntimeError`, PBKDF2's bounds with a `LogicError` before any HMAC, a public key off its curve at
  `read` and a low-order X25519 point at `agree`.
- **`jwe.vectors`** — `Core\Jwe::decrypt` answers `payload`, and `encrypt` handed the recorded
  `randomness` answers `token` byte for byte, which is what makes this the Novis → browser direction.
- **`jwe.refusals`** — every one refused, `policy` and `authenticity` alike, with the one `RuntimeError`.
- **`jws.keys`** — every key reads from `pkcs8`, `pem`, `spki` and `jwk` as a browser exported it, the
  forms agree, `write(KeyFormat::Jwk)` answers `jwkMinimal`, and `thumbprint` is base64url of its
  SHA-256.
- **`jws.keySets`** — `Jwt\KeySet::read` admits exactly `kids`, or refuses the whole set.
- **`jws.vectors`** — `verifyIssued`, at the crate-private level where the clock is an argument, answers
  each payload's claims, the structured ones included.
- **`jws.signs`** — `sign` at `clock` answers `token` byte for byte where `deterministic` is true, and
  otherwise the same header and payload segments and a signature that verifies — the Novis → issuer
  direction.
- **`jws.refusals`** — each refused with its recorded kind: `policy` and `authenticity` the one
  `RuntimeError`, `time` the expiry error, `claims` a refusal naming the claim.
- **`signatures.vectors`** — `Crypto::verify` accepts each recorded signature over its message under
  `jws.keys[key]`, and `Crypto::sign` reproduces it byte for byte where `deterministic` is true.
- **`signatures.refusals`** — each refused by `Crypto::verify` with the one `RuntimeError`.

Then `examples/webcrypto.nvs`, the example `rule:testing/feature-proofs` asks for, frozen by an `exact` check.
It opens one of the set's tokens, so the example is itself a browser's output being read. It also
verifies one of the set's ID tokens, and refuses the same token with its `alg` swapped.

**The file is never edited by a session.** If a vector disagrees with Novis, Novis is presumed wrong until
a published RFC vector says otherwise; a vector that is itself wrong is a finding for the handoff, and the
user regenerates the set by running `node tools/webcrypto-vectors.mjs`.

## Stage 8 — the rulebook

Flip `core-classes/crypto-interop-tier`, `security/jwe-compact-subset` and `security/jws-issued-subset`
to `shipped`, with `guardedBy` filled from this goal's cases and tests, and `python tools/rules.py
--render`.

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
  | `Crypto::generateKeyPair(Crypto\KeyKind $kind): Crypto\KeyPair` | P-256, X25519 or Ed25519; never RSA |
  | `Crypto::agree(Crypto\KeyPair $mine, Crypto\PublicKey $theirs): secret bytes` | the raw shared secret, meant for `expandKey` |
  | `Crypto\PublicKey::read(bytes $encoded, Crypto\KeyKind $kind, Crypto\KeyFormat $format): Crypto\PublicKey` | raw, SPKI or JWK, validated on read |
  | `$publicKey->write(Crypto\KeyFormat $format): bytes` | the same three forms |
  | `$publicKey->kind(): Crypto\KeyKind` | which kind it is |
  | `Crypto\KeyPair::read(secret bytes $pkcs8, Crypto\KeyKind $kind): Crypto\KeyPair` | a stored pair back, as DER or PEM |
  | `$keyPair->write(): secret bytes` | PKCS#8, so a server keeps its pair across requests |
  | `$keyPair->publicKey(): Crypto\PublicKey` | the half that is sent |
  | `Crypto::sign(bytes $message, Crypto\KeyPair $key): bytes` | the signature, its scheme the pair's kind |
  | `Crypto::verify(bytes $message, bytes $signature, Crypto\PublicKey $key): void` | nothing, or one `RuntimeError` |
  | `Jwe::encrypt(string $payload, secret bytes\|Crypto\PublicKey\|secret string $key): string` | the compact token |
  | `Jwe::decrypt(string $token, array<secret bytes>\|array<Crypto\KeyPair>\|secret string $keys): tainted string` | the payload, or one `RuntimeError` |
  | `Jwt::sign(array<string>\|object $claims, Duration $lifetime, secret bytes\|Crypto\KeyPair $key, {kid?, typ?, embedKey?}): string` | HS256 under a shared key, as today; RS256, PS256, ES256 or EdDSA from the pair's kind |
  | `Jwt::verifyIssued<T>(string $token, Jwt\KeySet\|Crypto\PublicKey $keys, string $issuer, string $audience, {leeway?, typ?, nonce?, maxAge?}): T` | a token another party issued, its claims as `T`, a `tainted` shape |
  | `Jwt\KeySet::read(tainted string $jwks, {rsaScheme?}): Jwt\KeySet` | a JWKS document the program fetched, admitted by the rules below |

  `Crypto\Cipher` is `XChaCha20Poly1305` and `Aes256Gcm`; `Crypto\KeyKind` is `P256`, `X25519`,
  `Ed25519`, `RsaPkcs1` and `RsaPss`; `Crypto\KeyFormat` is `Raw`, `Spki` and `Jwk`. Opening under the wrong cipher is a forgery, never
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
- **The roster, closed.** In: AES-256-GCM, PBKDF2-HMAC-SHA256, HKDF-SHA256, ECDH over P-256 and X25519,
  and four signature algorithms — RSASSA-PKCS1-v1_5 and RSASSA-PSS over SHA-256, ECDSA over P-256, and
  Ed25519. Internal only, never a member: AES Key Wrap (PBES2 needs it) and Concat KDF (ECDH-ES needs
  it). Out, and not to be added by any session of this goal: AES-CBC, AES-CTR, AES-128, RSA encryption in
  any form (RSA-OAEP, RSA1_5), RSA key generation, P-384 and ES384, RS384, RS512, PS384, PS512, ES256K,
  and JWE's `A*CBC-HS*` content encryption.
- **AES-GCM's sealed bytes are `nonce(12) ‖ ciphertext ‖ tag(16)`** — exactly what WebCrypto's `encrypt`
  answers with its IV put in front, so a browser splits at byte 12 and does nothing else. XChaCha's stay
  `nonce(24) ‖ ciphertext ‖ tag(16)`. The nonce is random and drawn through `crate::random::draw`, with no
  nonce parameter on the member; the record and `seal`'s doc state AES-GCM's 2^32-messages-per-key bound
  and name XChaCha as the answer when that bound matters. No additional-data parameter in this goal; JWE
  uses AAD internally.
- **Keys.** A symmetric key is `secret bytes`, length-checked: the wrong length is a `LogicError` naming
  the length wanted, never the bytes got (`crates/nvs-stdlib/src/crypto.rs:68-74`). A private key never
  leaves a `KeyPair` except as `secret bytes` through `write`. **Every public key is validated at
  `read`**: on the curve for P-256 — the set's off-curve point, the point at infinity and a short
  encoding are all refused — and an all-zero X25519 shared secret is refused at `agree`. A public key off
  the wire that fails is a `RuntimeError` (a verdict); a malformed key the program built is a
  `LogicError` (a bug). Two keys on different curves given to `agree` is a `LogicError`. `read` of a JWK
  accepts what a browser exports, `ext` and `key_ops` included, and ignores them; a JWK carrying `d` is
  refused, because a private key handed over as a public one is a bug. `write` answers RFC 7638's
  required members, sorted. JWK export of a private key is out of scope.
  **RSA and Ed25519 join on the same terms.** An RSA public key reads from SPKI or JWK — `Raw` is a
  `LogicError` — and one outside 2048–8192 bits is refused at `read`. An RSA pair reads from PKCS#8 and
  is never generated: `generateKeyPair` refuses both RSA kinds with a `LogicError` naming
  `KeyPair::read`, because `ring` generates none and a program is handed its RSA key by whoever issued
  it. **An RSA key's scheme is fixed when it is read** — `RsaPkcs1` or `RsaPss` — which is
  `rule:security/algorithm-comes-from-the-key` held for the one key type two JWS algorithms share.
  `KeyPair::read` accepts PKCS#8 as DER or as one PEM `PRIVATE KEY` block, for every kind — a
  service-account file carries exactly that — and refuses PKCS#1 and encrypted PKCS#8. An X25519 pair
  signs nothing and an Ed25519 or RSA pair agrees nothing: each is a `LogicError`.
- **PBKDF2 has a floor and a ceiling, and both are checked before the first HMAC.** The iteration count
  is a required parameter, because the other end chose it. It is refused below 100,000 and above
  2,000,000, and a salt shorter than 16 octets is refused. The vector set holds the refusal at each edge,
  so the record may not move either number without the user regenerating the set. The ceiling is what
  makes PBES2 safe: `p2c` in a JWE header is attacker-supplied, and without it one token buys unbounded
  CPU. PBES2's `p2c` and `p2s` are held to the same bounds.
- **JWE, the subset.** Compact serialization only. `enc` is `A256GCM` and nothing else. **The algorithm
  comes from the key's type** (`rule:security/algorithm-comes-from-the-key`): `secret bytes` means `dir`,
  a `PublicKey` or `KeyPair` means `ECDH-ES` (direct agreement, no key wrap, empty `apu` and `apv`), and a
  `secret string` password means `PBES2-HS256+A128KW`. The header's `alg` and `enc` are read only to be
  **compared**, as `crates/nvs-stdlib/src/jwt.rs:11-22` does, and a mismatch is a refusal. The allowed
  header parameters are `alg`, `enc`, `epk`, `p2s`, `p2c`, `kid`, `typ` and `cty`. `zip`, `crit`, `jku`,
  `x5u`, `x5c`, a `jwk` other than `epk`, and anything unknown are refused, because `zip` is a
  decompression bomb and the URL-bearing ones are fetches. The protected header is length-capped before
  it is parsed. **`encrypt` writes its header canonically** — members sorted, no whitespace, at every
  level — which is the form the vector set's tokens are in and what lets `encrypt` be held to them byte
  for byte.
- **JWE answers like the rest of the roster.** The payload is `string` in and **`tainted string`** out,
  as with `Core\Signature::verify`, and every refusal is one `RuntimeError` with one sentence. Decrypt's
  key ring is tried in order, as `Core\Signature::verify`'s is, except that a password is exactly one,
  because every try costs a full derivation.
- **JWS, the subset.** Compact serialization only. **The algorithm comes from the key**: a shared key is
  HS256, and a pair or public key of kind `P256`, `Ed25519`, `RsaPkcs1` or `RsaPss` is ES256, EdDSA,
  RS256 or PS256. The header's `alg` is read only to be compared, as `crates/nvs-stdlib/src/jwt.rs:11-22`
  does today.
  - **`sign`** writes its header canonically — `{alg, jwk?, kid?, typ}`, sorted, no whitespace, `typ`
    `JWT` unless the bag names another — and its payload in today's layout: the caller's claims in
    order, then `iat`, then `exp` (`crates/nvs-stdlib/src/jwt.rs:364-425`). Under RS256 and EdDSA a
    token is therefore reproducible byte for byte. `embedKey` writes the pair's public half as RFC 7638's
    minimal JWK under `jwk`, which is what a DPoP proof carries, and is a `LogicError` under a shared
    key. `exp` stays mandatory on everything `sign` writes (`rule:security/jwt-expiry-is-mandatory`).
    **Structured claims** — a shape or a deriving class, and only under a key pair — are written as
    `Core\Json::encode` writes them, with `iat` and `exp` appended in that order; `iat` or `exp` among
    them is a `LogicError`, as it is today, and a `secret` field is refused, as `encode` refuses one. The
    record fixes whether this is a union on `sign` or a second member.
  - **`verifyIssued`** refuses a header carrying `jku`, `x5u`, `x5c`, `jwk`, `crit`, `b64`, `zip` or
    `cty` — a key or a fetch the token brings, an extension, an unencoded or compressed payload, a nested
    token — and ignores every other member it does not read. That is looser than JWE on purpose: an
    issuer sends hints such as `x5t`, and a verifier refusing them refuses real ID tokens. The token is
    length-capped before it is parsed.
  - **A key is found, never tried.** `kid` is a lookup into the set and selects nothing else; a token
    without one verifies only against a set of exactly one key. There is no try-every-key, so a token
    costs at most one signature check.
  - **The clock.** `exp` is required and `nbf` is checked when present, both under `leeway`: a
    `Duration`, `60s` when omitted, and a `LogicError` when negative or above `5m`. A token is accepted
    while `now < exp + leeway` and `now ≥ nbf − leeway`.
  - **The claims.** `iss` equals `$issuer`. `aud` is `$audience` or a list holding it, and a list of more
    than one requires `azp` equal to `$audience`. A `nonce` asked for is compared in constant time, and
    an absent one is refused. `typ` is compared case-insensitively with any `application/` prefix
    removed. `maxAge` requires an `auth_time` no older than `maxAge + leeway`.
  - **The order, and what each refusal says.** Shape, header policy, key, signature — then the clock and
    the claims, which are only reached under a signature that held. Policy and authenticity are the one
    `RuntimeError` sentence. Expiry keeps its own message, for the reason
    `crates/nvs-stdlib/src/jwt.rs:57-64` gives, and a claims refusal names the claim, safe for the same
    reason.
  - **`Jwt\KeySet::read`** skips a key marked `use: enc` and a key whose `alg` or kind is outside the
    roster, and refuses the whole set for a private member (`d`, `p`, `q`, `dp`, `dq`, `qi`, `k`), an
    RSA key under 2048 bits, two keys under one `kid`, an `alg` its kind cannot carry, a document that is
    not JSON, and an RSA key carrying no `alg` when no `rsaScheme` was named. `rsaScheme` is `RsaPkcs1`
    or `RsaPss` and reaches only an RSA key carrying no `alg`; a key's own `alg` wins. The record fixes
    the set's key-count cap.
  - **Fetching, caching and discovery are not `Core`.** `rule:security/protocol-admission-test` puts the
    flow in a package, and every member here is stateless over a key it is handed.
- **Claims arrive as a shape.** `verifyIssued<T>`'s `T` is held to
  `rule:security/derived-codec-qualifiers` at the call, exactly as `Core\Request::jsonAs<T>`'s is: an
  inline shape must be `tainted {…}`, a class must declare `tainted` on every text field reachable from
  it, and anything else is a diagnostic naming the field. `rule:security/verification-does-not-launder`
  is kept by the type rather than by flattening every claim to text. The registered claims are checked
  before `T` is decoded, and `T` may declare them to read them. `Jwt::verify` keeps its
  `array<tainted string>` answer and its refusal of structured claims, and `sign` writes structured
  claims only under a key pair — a token signed for another party — so a token this program signs under
  a shared key still carries only what `verify` reads back.
- **Raw signatures.** `Crypto::sign` answers the signature octets: RSASSA-PKCS1-v1_5 or RSASSA-PSS over
  SHA-256 by the RSA kind, ECDSA P-256 over SHA-256 as the 64-octet `r ‖ s` that JWS and WebCrypto both
  use — never DER — or Ed25519. `Crypto::verify` answers nothing or throws the one `RuntimeError`
  `rule:security/verification-throws-and-compares-in-constant-time` asks for, whatever was wrong, and
  never a `bool`. An X25519 pair signs nothing: a `LogicError`. The message is `bytes` in both — checking
  what a peer sent is the point — and verifying launders nothing. Standard Webhooks' `v1a` and HTTP
  Message Signatures (RFC 9421) are package code over these two members.
- **Dependencies, all pure Rust, all on the generation already locked**: `aes-gcm` 0.11 (brings `aes`
  0.9), `aes-kw` 0.3, `p256` 0.14 with its `ecdh` feature, `x25519-dalek` 3.0, `hkdf` 0.13, and `pbkdf2`
  0.13 as a direct dependency. Default features are off and no `getrandom` anywhere: randomness reaches
  them through an adapter over `crate::random::draw`. **`p256`'s curve arithmetic has never been
  independently audited** (its README says so); the record names that and accepts it, because P-256 is
  the one curve every browser's WebCrypto has and the crate is written constant-time. No new C crypto
  crate: `deny.toml:70-73` bans OpenSSL and `aws-lc-rs` stays off. **`ring` is the one exception, and it
  is not new**: it is already linked as TLS's provider (`Cargo.toml:215-236`), and this goal adds it to
  `nvs-stdlib` directly, at the version the lockfile already resolves, for the four signature algorithms
  and nothing else — every AEAD, KDF and key agreement stays RustCrypto. Stage 3 widens its recorded
  answer under `rule:packaging/a-c-dependency-answers-two-questions` from TLS to JWS on the same two
  questions: attacker bytes reach a token verifier by construction, and BoringSSL's record is the same
  record. PEM is read with what the graph already carries (`crates/nvs-host/src/tls.rs:121`). Each added crate
  regenerates `THIRD-PARTY-LICENSES.txt` in the same commit
  (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`). If a pinned version will
  not build against the locked base, take the nearest one that does and say so in the commit.
- **The interop proof is WebCrypto's own output, frozen.** `tools/webcrypto-vectors.mjs` wrote the set
  from Node's `crypto.subtle` — the W3C API a browser ships — with every input derived from a label, so a
  rerun writes the same bytes and `--check` says whether the file is current. It reproduces RFC 7518
  Appendix C's Concat KDF output before writing, and its JWE tokens were opened by `jose`, an independent
  JOSE implementation, when the set was made. Node is needed only to regenerate the set, which the user
  fires; no check and no session runs the script. The set's `jws` section is the same proof for
  signatures: WebCrypto signed every token, its RSA keys are built from label-derived primes because
  WebCrypto derives none from a seed, and an ES256 or PS256 signature — randomized by its algorithm — is
  kept from the file on disk while it still verifies, so a rerun still writes the same bytes. The
  script's `referee` judges every JWS vector and refusal by the rules above before the file is written.
- **What it spends**: per call, a few hundred bytes of cipher or curve state on the stack and one output
  buffer charged to the request through `nvs_runtime::budget`, passed through `nvs_runtime::affordable`
  first, as `seal` is today. A `KeyPair` holds its private scalar for as long as the program holds the
  object. PBKDF2 is bounded in CPU by its ceiling. Nothing is held between calls, so it is O(in-flight).
  `verifyIssued` is at most one signature check per token — RSA verification is the dearest, and the
  key-count cap and the no-try rule hold it to one — and a `Jwt\KeySet` holds its public keys for as long
  as the program holds it. `Core` caches no key set.
- **Not this goal**: JWK export of private keys, an additional-data parameter, streaming encryption, the
  JWS JSON serialization, a detached or unencoded JWS payload, JWE-encrypted ID tokens and RSA-OAEP with
  them — skipped by the user as rare and opt-in at every provider — fetching or caching a key set, and
  structured claims under a shared key. A session that finds one on its path writes it to the handoff's
  `## Backlog`.
