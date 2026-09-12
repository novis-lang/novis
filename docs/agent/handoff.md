# Handoff

## State

**Goal `webcrypto` — stage 4's member half is closed.** `Core\Crypto`
(`crates/nvs-stdlib/src/crypto.rs:629`) now carries `sign` and `verify` beside `generateKeyPair`,
`agree` and the five AEAD and KDF rows, so every member of the goal's surface table below `Jwe` and
`Jwt` is registered. Nothing in `crypto.rs` carries an `expect(dead_code)` for a signature any more:
`SigningKey`, `PrivateKey::signing`, `sign`, `VerifyingKey`, `PublicKey::verifying` and
`verify_signature` are all reached by a row. The markers that remain are `wrap_key`'s and the JWE
half's, which stages 5 and 6 take.

**The interop claim is now asserted from Novis rather than from Rust.** Under RS256 and EdDSA this
runtime writes WebCrypto's own signature byte for byte, over all seven deterministic vectors of the
frozen set's `signatures` section; ES256 and PS256 draw per signature, so those six are held to
verifying rather than to their bytes, and all thirteen of WebCrypto's signatures verify here.

**What stage 4 still owes is prose only.** `docs/reference/core/Crypto.md` is still the AEAD-only
page, `crypto.rs`'s module doc still has the four sentences stage 0 lists, and
`docs/spec/02-php-migration.md`'s two derivation rows still name no member. Nothing gates any of
them, which is why they are the group below.

## Next group

**Stage 4: the prose half, which is all that is left of the stage** — one file set: the module doc
and reference page over the surface that just landed, plus the two migration rows that name its
members.

- [ ] **`docs/reference/core/Crypto.md` rewritten for the whole surface** — its summary and opening
      (`docs/reference/core/Crypto.md:2`) still say "no cipher, mode, padding or nonce argument" and
      "three members and no cipher name anywhere in them", which the required `Cipher` enum and the
      ten members now on the class both falsify. Then `python tools/reference.py` regenerates
      `docs/novis.md` from it, in the same commit. `rule:core-api/reference-card`.
- [ ] **The four module-doc sections stage 0 names, each rewritten whole** —
      `crates/nvs-stdlib/src/crypto.rs:1` ("three members that take a key and a message and nothing
      else"), `:8` (§ *No cipher argument*, whose argument was against a cipher named as a string),
      `:36` ("AES-GCM was the alternative and loses on two counts", now why XChaCha is the one to
      *prefer*), and `:103` ("there is exactly one `XChaCha20Poly1305::new_from_slice`"). The goal's
      stage 0 list says what each becomes; `docs/agent/conventions.md` § *A code comment* is the
      no-changelog rule they are rewritten under.
- [ ] **`docs/spec/02-php-migration.md:811`** — `hash_hkdf` is a `member` row naming no member and
      `hash_pbkdf2` sends a key rather than a password, which the goal's stage 4 paragraph says
      become two rows naming `deriveKey` and `expandKey` once the members are registered. They are.

## Backlog

- `examples/webcrypto.nvs` — the acceptance fixture, still unwritten; it is stage 7's.
- Standard Webhooks' `v1a` and RFC 9421 signatures are package code over `sign`/`verify`, not `Core`
  (goal § *Standing decisions*, *Raw signatures*).
- `Jwt::sign` over a key pair and `Jwt\KeySet` are stage 6's, over these two members' primitives.
