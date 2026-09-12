# Handoff

## State

**Goal `webcrypto` — stage 4's key half is closed.** `Core\Crypto`
(`crates/nvs-stdlib/src/crypto.rs:629`) now carries `generateKeyPair` and `agree` beside its five
AEAD and KDF rows, with three `.nvst` cases each. `generated_pkcs8`
(`crates/nvs-stdlib/src/crypto.rs:2702`) owns how a drawn key becomes a file: the two RFC 8410 kinds
are a 16-octet constant prefix in front of a scalar drawn through `crate::random::draw`, and P-256
goes through `p256`'s `EncodePrivateKey`, which the locked `pkcs8` + `alloc` features do carry. A
drawn pair is the same object a read pair is, so `write` hands back a file `KeyPair::read` takes.

**`agree` (`crates/nvs-stdlib/src/crypto.rs:2781`) draws the refusal line by who made the mistake.**
`NoAgreement::Keys` is a `LogicError` — a kind that agrees nothing, or two keys on different curves,
both of them the program's own doing — and `NoAgreement::Point` is a `RuntimeError`, the verdict on a
peer whose X25519 point contributes nothing. `pair_of` (`:3307`) and `key_of` (`:3163`) each take a
slot index now, since `agree`'s two keys are arguments rather than a receiver.

**What stage 4 still owes** is `Crypto::sign` and `Crypto::verify`: `SigningKey`, `PrivateKey::signing`
and `sign` still carry `expect(dead_code)` placeholders, and so do `VerifyingKey`'s side.
`docs/reference/core/Crypto.md` is still the AEAD-only page — nothing gates it, and stage 4's table
says it is rewritten for the new surface.

## Next group

**Stage 4: the two raw signature rows** — one file set: `crates/nvs-stdlib/src/crypto.rs` and new
`tests/conformance/core/crypto-*.nvst` cases. Both rows go on `CLASS`
(`crates/nvs-stdlib/src/crypto.rs:629`), which is registered already; `nvs_core_crypto_agree`
(`crates/nvs-stdlib/src/crypto.rs:3026`) is the worked example for a row taking a key object as an
argument, and the frozen set's `signatures` section is what a case asserts against.

- [ ] **`Crypto::sign(bytes $message, Crypto\KeyPair $key): bytes`** — over `PrivateKey::signing`
      (`crates/nvs-stdlib/src/crypto.rs:2571`) and `sign` (`crates/nvs-stdlib/src/crypto.rs:2806`),
      with `pair_of(args, 1, "sign")` (`crates/nvs-stdlib/src/crypto.rs:3307`) as the reader. An
      X25519 pair signs nothing and is a `LogicError`, which is what `signing`'s `None` already
      means. ECDSA answers the 64-octet `r ‖ s` and never DER. Removing the three
      `expect(dead_code)` placeholders this reaches is part of the slice.
      `rule:security/algorithm-comes-from-the-key`, `rule:core-api/shape-rules`.
- [ ] **`Crypto::verify(bytes $message, bytes $signature, Crypto\PublicKey $key): void`** — over
      `PublicKey::verifying` (`crates/nvs-stdlib/src/crypto.rs:1897`) and `verify_signature`
      (`crates/nvs-stdlib/src/crypto.rs:2445`), with `key_of(args, 2, "verify")`
      (`crates/nvs-stdlib/src/crypto.rs:3163`). It answers nothing or one `RuntimeError` and never a
      `bool`; an X25519 key is the one `LogicError`.
      `rule:security/verification-throws-and-compares-in-constant-time`.
- [ ] **Three `.nvst` cases per row**, the two members asserted together: `RS256` and `EdDSA`
      reproduce the frozen signature octet for octet, while `ES256` and `PS256` are randomized and so
      are held to verifying rather than to their bytes. The split is the module doc's *four
      signatures* section (`crates/nvs-stdlib/src/crypto.rs:210`), and `verify_signature`
      (`crates/nvs-stdlib/src/crypto.rs:2445`) is the one comparison either half reaches.

## Backlog

- `examples/webcrypto.nvs` — the goal's acceptance fixture, which no stage has written yet; the
  driver reports it missing after every session. `docs/agent/loop-goal.md`.
- `docs/reference/core/Crypto.md` is still the AEAD-only essay, with no paragraph on the key classes,
  `generateKeyPair` or `agree`. Stage 4's table says it is rewritten for the new surface.
- The standing decisions say PEM is read "with what the graph already carries
  (`crates/nvs-host/src/tls.rs:121`)"; that reader is `rustls`'s and `nvs-stdlib` cannot reach it —
  `pem-rfc7468` is not in the lockfile, so `der`'s own is off too. `pkcs8_der` decodes the armour
  itself. Worth correcting in the record when stage 4 is recorded.
- `Core\Jwe` and `Jwt::verifyIssued` are still unregistered in
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`; both are later stages of this
  goal.
