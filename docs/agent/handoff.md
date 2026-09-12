# Handoff

## State

**Goal `webcrypto` — stage 4's `Crypto\PublicKey` is a registered class; `Crypto\KeyPair` is not.**
`crate::crypto::PUBLIC_KEY` (`crates/nvs-stdlib/src/crypto.rs:885`) declares `read` as a static and
`write`/`kind` as instance members, registered by one line in `registry::CLASSES`
(`crates/nvs-stdlib/src/registry.rs:1712`) and by no `lib.rs` edit at all. Three conformance cases
under `tests/conformance/core/crypto-public-key-*.nvst` hold it. `Core\Crypto` itself still has only
its five AEAD and KDF rows (`crates/nvs-stdlib/src/crypto.rs:617`).

**The design question stage 4 asked is answered and on disk.** The slots are the canonical SPKI and
the kind's `KEY_KIND` constant, and every member goes back through `PublicKey::read`; `PUBLIC_KEY`'s
own doc is the home of why, and `key_of` (`crates/nvs-stdlib/src/crypto.rs:2598`) is the reader.
`PublicKey::kind` is therefore still `expect(dead_code)`: the member answers from the slot, and what
will reach the Rust method is a signature check picking its scheme or an agreement refusing two
curves.

**What the next session must not assume.** `read_signing_key` (`crates/nvs-stdlib/src/crypto.rs:2282`)
takes a `SignatureKind` (`:2238`), whose cases are the roster's four **signing** kinds — there is no
`X25519` in it, because an X25519 pair signs nothing. So a `Crypto\KeyPair` covering all five kinds
cannot be `read_signing_key` with a class around it: the X25519 private-key path, `write()` back to
PKCS#8, and `publicKey()` for every kind are new codec work, not registration work. Nothing is
blocked; this is scope, not a decision.

## Next group

**Stage 4: the key pair and the two `Core\Crypto` rows** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, `crates/nvs-stdlib/src/registry.rs` and new
`tests/conformance/core/crypto-key-pair-*.nvst` cases. `PUBLIC_KEY`
(`crates/nvs-stdlib/src/crypto.rs:885`) is now the worked example for all of it — a `CoreClass` with
`methods:`, `instance:` and `slots:` together, its cards beneath it, `key_of` (`:2598`) for the slot
reader and `nvs_core_crypto_public_key_read` (`:2634`) for a static that answers an instance. A
second class in this module still costs no `lib.rs` edit and one line in `registry::CLASSES`
(`crates/nvs-stdlib/src/registry.rs:1712`). Registering a class deletes its line from
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`, which otherwise fails as a stale
line.

- [ ] **`Crypto\KeyPair` becomes a class** — `read(secret bytes $pkcs8, Crypto\KeyKind $kind)` as a
      static, `write()` and `publicKey()` as instance members, over `read_signing_key`
      (`crates/nvs-stdlib/src/crypto.rs:2282`) widened past `SignatureKind`'s four signing cases
      (`:2238`) to carry X25519 too. Decide there whether the slots hold the PKCS#8 and re-read, as
      `PUBLIC_KEY` (`:885`) does, or take `crate::regex`'s per-core cache that
      `crates/nvs-stdlib/src/instance.rs:17` names — and if the first, rewrite `SigningKey`'s doc
      claim that a pair holds its parsed key across calls (`crates/nvs-stdlib/src/crypto.rs:2257`).
      `rule:core-classes/crypto-interop-tier`, `rule:core-api/shape-rules`.
- [ ] **`generateKeyPair` and `agree` become rows on `Core\Crypto`** — beside the five at
      `crates/nvs-stdlib/src/crypto.rs:617`, over `agree_x25519` (`:1364`) and `agree_p256` (`:1400`),
      with `generateKeyPair` refusing both RSA kinds as a `LogicError` naming `KeyPair::read`.
      `rule:core-classes/crypto-interop-tier`.

## Backlog

- `examples/webcrypto.nvs` is the acceptance fixture the driver reports missing every session; it
  needs the whole stage-4 surface before it can be written — `docs/agent/loop-goal.md`.
- `Crypto\PublicKey::write` answers a plain `bytes` from a key read out of `tainted` octets. The
  standing decisions fix that signature, and the octets are a re-encoded validated key rather than
  anything the sender chose, but no rule states the reading — `rule:security/verification-does-not-launder`.
- JWK export of a private key stays out of this goal — `docs/agent/loop-goal.md` § *Not this goal*.
