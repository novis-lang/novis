# Handoff

## State

**Goal `webcrypto` — Novis reads what a browser encrypts and what an issuer signs. Stage 3 is
everything but the signatures: each AEAD, derivation, agreement and wrap the roster names is on disk
crate-private, under its published vectors.** Goal `template-format`'s list is this goal's Stage 1 floor
and is green; Stage 2 is [ADR 0179](../decisions/0179.md), which is the design and the one home for it.

In `crates/nvs-stdlib/src/crypto.rs`: the two ciphers, `derive_key`/`expand_key`, `agree_x25519` and
`agree_p256` behind `read_p256_point`, and `wrap_key`/`unwrap_key`/`concat_kdf`, which are JWE's two
internal pieces and never members. No `Core` member is registered for any of it — that is stage 4 — so
each carries `#[cfg_attr(not(test), expect(dead_code, …))]`, which fails the day stage 4 lands and is
how the marker deletes itself.

`crates/nvs-stdlib/src/tests/vectors.rs` is new — `crate::tests::vectors`, declared inside `lib.rs`'s
own test module for the reason the playbook's new bullet gives: the one reader of the frozen WebCrypto
set, so an interop expectation is asserted against the file rather than copied into a test. Its module
doc is the home of which kind of vector is read and which stays inline. Nothing is blocked.

## Next group

**Stage 3: the four signature algorithms through `ring`** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, appended after `concat_kdf` at
`crates/nvs-stdlib/src/crypto.rs:911`, with the tests beside the five already there and the vector set
reached through `crate::vectors` the way the agreement tests reach it.

Three facts about `ring` are already read out of its source, so no slice below has to discover them:
its `rand::SecureRandom` is sealed by a `pub(crate)` supertrait (`ring 0.17.14 src/rand.rs:23,62`), so
`SystemRandom` is the only rng that can be handed in and PS256 and ES256 draw outside
`crate::random::draw`'s seam — `RsaKeyPair::sign` (`src/rsa/keypair.rs:533`) and `EcdsaKeyPair::sign`
(`src/ec/suite_b/ecdsa/signing.rs:173`) each take one and `Ed25519KeyPair::sign` takes none; WebCrypto
exports Ed25519 as PKCS#8 **v1**, so `from_pkcs8_maybe_unchecked` (`src/ec/curve25519/ed25519/signing.rs:111`)
and not `from_pkcs8`; and `EcdsaKeyPair::from_pkcs8` requires the optional `[1] publicKey`
(`src/ec/suite_b.rs:199`) that WebCrypto's P-256 export omits — the set's `/jws/keys/ec-1/pkcs8` is 67
octets — so ES256 signing needs `from_private_key_and_public_key` with the point derived through
`p256`.

- [ ] **Verification for all four, from the public material a JWK carries** — appended at
      `crates/nvs-stdlib/src/crypto.rs:911`: RSA from its `n` and `e` through
      `RsaPublicKeyComponents`, P-256 from `04 ‖ x ‖ y` and Ed25519 from `x`, each with its scheme
      given by the key rather than by anything read off a token
      (`rule:security/algorithm-comes-from-the-key`), answering nothing or a refusal and never a
      `bool` at the member above it (`rule:security/verification-throws-and-compares-in-constant-time`).
      The set's `signatures` section is the vector: 13 signatures and 7 refusals, the DER-form ES256
      among them, since `r ‖ s` is the only form.
- [ ] **Signing the deterministic pair, RS256 and EdDSA** — appended at
      `crates/nvs-stdlib/src/crypto.rs:911`, from PKCS#8 and asserted byte for byte against the set's
      vectors carrying `deterministic: true`, which is what pins this runtime's token bytes to
      WebCrypto's. The goal's § *Standing decisions* under *Raw signatures* is the surface they serve.
- [ ] **Signing the randomized pair, PS256 and ES256, and the module doc sentence about their
      randomness** — appended at `crates/nvs-stdlib/src/crypto.rs:911`, ES256 as the 64-octet `r ‖ s`
      and never DER, asserted by verifying what they signed rather than against fixed bytes, with the
      first `ring` fact above written into the module doc where the nonce seam is explained.

## Backlog

- `crates/nvs-stdlib/src/crypto.rs` is 1482 lines and the signature half adds several hundred more;
  whether it moves to a module of its own is the next group's first decision rather than something it
  discovers — `docs/agent/playbook.md` § *Splitting a file that got too big*.
- Stage 3's last row, the JWK thumbprint — `docs/agent/loop-goal.md` § *Stage 3* has its vector.
- Structured claims under a *shared* key stay out of this goal — ADR 0179 § *Consequences*.
- JWE-encrypted ID tokens and RSA-OAEP with them, skipped by the user — ADR 0179 § *Alternatives
  rejected*.
- `Core\Signature` is still the roster's one entry with no member, module or case —
  `rule:core-classes/signature`.
