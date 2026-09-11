---
milestone: M8
---
# Loop goal 47 — a browser and a Novis server read each other's encrypted data

Anything a browser encrypts with WebCrypto, a Novis server can decrypt, and the other way round, with no
JavaScript crypto library on the browser side. `Core\Crypto` gains an **interop tier** beside the
XChaCha20-Poly1305 construction it already has — AES-256-GCM, PBKDF2-HMAC-SHA256, HKDF-SHA256 and ECDH
over P-256 and X25519, each a separately named member — and the protocol roster gains **`Core\Jwe`**, a
closed subset of JSON Web Encryption (RFC 7516/7518) built on those primitives. What proves it is Node's
own `crypto.subtle`, which is the W3C Web Cryptography API browsers expose, round-tripping every mode
against the built `nvs` binary.

Nothing is removed. `Core\Crypto::generateKey`, `seal` and `open` keep their construction, their
signatures and their bytes, and every `Core\Digest` case stays. XChaCha20-Poly1305 remains the
construction a Novis program reaches for when both ends are Novis; the interop tier is for when one end
is a browser.

## Why here

Directly after goal `template-format`, on the user's call rather than a dependency: it shares no file with
that goal (`nvs-lsp`, `editors/vscode`) or with the others near it, so its position costs nothing and
moves nothing. It opens `nvs-stdlib` and the workspace manifest and nothing else.

Before goal `gap-zero`, for that goal's standing reason — a register is emptied after everything that
adds to it has run, and this goal adds and closes rows.

What it needs already built, all on disk: `Core\Crypto`'s one construction and its three seams
(`crates/nvs-stdlib/src/crypto.rs:103-113`), every draw through `crate::random::draw`
(`crates/nvs-stdlib/src/crypto.rs:115-121`), JWT's compare-only header reading that JWE copies
(`crates/nvs-stdlib/src/jwt.rs:11-22`), `Core\Signature`'s `tainted` payload answer
(`docs/spec/01-core-library.md:1175`), base64url in `crates/nvs-stdlib/src/encoding.rs`, and the RustCrypto
base the lockfile already carries — `aead` 0.6, `cipher` 0.5, `hmac` 0.13, `sha2` 0.11, `pbkdf2` 0.13
and `curve25519-dalek` 5.

## Stage 0 — the catch-up

Sentences on disk this goal makes wrong. Each is corrected in the stage that makes it wrong. Re-grep
before editing: these are anchors, and files move.

- `crates/nvs-stdlib/src/crypto.rs:1-2` — "as three members that take a key and a message and nothing
  else". Stage 4.
- `crates/nvs-stdlib/src/crypto.rs:36-40` — "AES-GCM was the alternative and loses on two counts". Still
  true as the reason XChaCha is the *default*; rewritten in stage 4 to say that, and that AES-GCM is here
  for interop. Rewritten as a whole, never as a sentence added beside the old one.
- `crates/nvs-stdlib/src/crypto.rs:103-113` — "there is exactly one `XChaCha20Poly1305::new_from_slice`".
  Stage 4 says what the second construction is and that it has one home too.
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

## Stage 2 — the record

One new record, and no other number. It creates `core-classes/crypto-interop-tier` and
`security/jwe-compact-subset`, both `designed`, and modifies `security/protocol-roster`. Its body is
§ *Standing decisions* below, argued: this is transcription, not design. It fixes the members'
spellings, which the standing decisions leave to it under `rule:core-api/verb-lexicon`, and the numbers
it tunes stay inside the bounds given there.

The spec rows move with it: `docs/spec/01-core-library.md:1173` gains the interop tier, and a `Core\Jwe`
row joins § 16. The migration rows wait for stage 4 (the handoff says why).

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

## Stage 4 — the `Core\Crypto` members

The interop tier as `Core` members, five edits each (conventions.md § *A `Core` member*): AES-256-GCM
seal and open, PBKDF2 and HKDF derivation, a key pair for each curve and the agreement between two, and
public keys read and written in raw, SPKI and JWK form. Then `docs/spec/02-php-migration.md:811-812`
becomes two `member` rows naming them, which `every_migration_member_row_names_a_registered_member` and
`every_migration_member_row_has_a_conformance_case` hold. Stage 0's three `crypto.rs` sentences are
rewritten here.

## Stage 5 — `Core\Jwe`

A new `crates/nvs-stdlib/src/jwe.rs`, registered in `crates/nvs-stdlib/src/registry.rs` beside
`crate::jwt::CLASS` (`:1706`): compact serialization, the subset the standing decisions fix, encrypt and
decrypt, over stage 4's crate-private functions — never a second copy of a primitive.

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

- **Additive, and nothing shipped changes.** `generateKey`, `seal` and `open` keep their names,
  signatures, output bytes and error text, and their six conformance cases are not edited. Every
  `Core\Digest` case stays, the broken ones included. **Streamlining is allowed only inside the crate** —
  one key-length `LogicError` shape, one forgery sentence, one nonce-draw path shared by both
  constructions. A shipped member's observable behaviour is never the price of it.
- **The roster, closed.** In: AES-256-GCM, PBKDF2-HMAC-SHA256, HKDF-SHA256, ECDH over P-256 and X25519.
  Internal only, never a member: AES Key Wrap (PBES2 needs it) and Concat KDF (ECDH-ES needs it). Out,
  and not to be added by any session of this goal: AES-CBC, AES-CTR, AES-128, RSA in any form, and JWE's
  `A*CBC-HS*` content encryption. ECDSA and Ed25519 signatures are signing, not this goal.
- **No cipher-name-as-string, anywhere.** Each primitive is its own member, as `Core\Hash::hmac` is not
  and `Core\Jwt` is. An enum argument naming a curve is acceptable, because it is a closed set of values
  and not a string. The record picks the spellings. If it cannot settle them, the fallback is
  `sealAesGcm`/`openAesGcm`, `deriveKeyPbkdf2`, `expandKeyHkdf`, `generateKeyPair(Crypto\Curve)` and
  `agree`, recorded as the record's choice.
- **AES-GCM's sealed bytes are `nonce(12) ‖ ciphertext ‖ tag(16)`** — exactly what WebCrypto's `encrypt`
  answers with its IV put in front, so a browser splits at byte 12 and does nothing else. The nonce is 96
  random bits drawn through `crate::random::draw`, with no nonce parameter. The record and the member's
  doc state the 2^32-messages-per-key bound this implies, and name XChaCha as the answer when that bound
  matters. No additional-data parameter in this goal; JWE uses AAD internally.
- **Keys.** A symmetric key is `secret bytes`, length-checked: the wrong length is a `LogicError` naming
  the length wanted, never the bytes got (`crates/nvs-stdlib/src/crypto.rs:68-74`). A private EC key is
  `secret bytes`, generated by Novis or imported raw or as PKCS#8. A public key is plain `bytes`, read and
  written as raw (65-octet uncompressed SEC1 for P-256, 32 octets for X25519), SPKI DER and JWK text.
  **Every public key is validated at import**: on the curve for P-256, and an all-zero X25519 shared
  secret is refused. A public key off the wire that fails validation is a `RuntimeError` (a verdict),
  while a malformed key the program itself built is a `LogicError` (a bug). JWK export of a private key
  is out of scope.
- **PBKDF2 has a floor and a ceiling, and both are checked before the first HMAC.** The iteration count
  is a required parameter, because the other end chose it. It is refused below 100,000 and above
  2,000,000, and a salt shorter than 16 octets is refused. The record may move either number with a
  reason, never remove one. The ceiling is what makes PBES2 safe: `p2c` in a JWE header is
  attacker-supplied, and without it one token buys unbounded CPU.
- **JWE, the subset.** Compact serialization only. `enc` is `A256GCM` and nothing else. **The algorithm
  comes from the key** (`rule:security/algorithm-comes-from-the-key`): a 32-octet `secret bytes` means
  `dir`, a P-256 or X25519 key means `ECDH-ES` (direct agreement, no key wrap), and a `secret string`
  password means `PBES2-HS256+A128KW`. The header's `alg` and `enc` are read only to be **compared**, as
  `crates/nvs-stdlib/src/jwt.rs:11-22` does, and a mismatch is a refusal. The allowed header parameters
  are `alg`, `enc`, `epk`, `p2s`, `p2c`, `kid`, `typ` and `cty`. `zip`, `crit`, `jku`, `x5u`, `x5c`, a
  `jwk` other than `epk`, and anything unknown are refused, because `zip` is a decompression bomb and the
  URL-bearing ones are fetches. The protected header is length-capped before it is parsed.
- **JWE answers like the rest of the roster.** The payload is `string` in and **`tainted string`** out,
  as with `Core\Signature::verify`, and every failure of authenticity is one `RuntimeError` with one
  sentence (`crates/nvs-stdlib/src/crypto.rs:68-74`). Decrypt takes the key-ring shape
  `Core\Signature::verify` takes, tried in order, except that a PBES2 ring holds exactly one password,
  because every try costs a full derivation.
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
  first, as `seal` is today. PBKDF2 is bounded in CPU by its ceiling. Nothing is held between calls, so
  it is O(in-flight).
- **Not this goal**: JWS with ES256 or EdDSA, JWK export of private keys, an additional-data parameter,
  streaming encryption. A session that finds one on its path writes it to the handoff's `## Backlog`.
