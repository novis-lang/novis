# Handoff

## State

**Goal `webcrypto` — stage 4's two key classes are both registered.** `Crypto\KeyPair`
(`crates/nvs-stdlib/src/crypto.rs:1044`) declares `read` as a static and `write`/`publicKey` as
instance members, over slots that are the PKCS#8 and the kind; its own doc owns why those and why
there is no cache of parsed keys behind them. Three `.nvst` cases under
`tests/conformance/core/crypto-key-pair-*.nvst` hold it. `Core\Crypto` itself
(`crates/nvs-stdlib/src/crypto.rs:630`) still has only its five AEAD and KDF rows, and closing that
is the whole of what stage 4 has left.

**The codec now reads all five kinds.** `PrivateKey` (`crates/nvs-stdlib/src/crypto.rs:2379`)
replaced `SignatureKind` and `read_signing_key`: it reads every kind of the roster, answers the
public half with `public()`, and narrows to the four that sign through `signing()`, so `SigningKey`
has exactly one producer and X25519 reaches no signer. `pkcs8_der`
(`crates/nvs-stdlib/src/crypto.rs:2548`) unwraps one PEM `PRIVATE KEY` block and `x25519_scalar`
(`:2522`) walks RFC 8410's structure — the one kind neither `p256` nor `ring` reads a private key
for.

**What the next session must not assume.** Nothing in this module *writes* a PKCS#8. `write`
answers the slot, and `KeyPair::read` stores what it was handed, so `generateKeyPair` is the first
member that has to encode one — see the next group's first item. `agree_p256` and `agree_x25519`
still carry `expect(dead_code)`, which registering `agree` removes.

## Next group

**Stage 4: the two rows that close `Core\Crypto`** — one file set:
`crates/nvs-stdlib/src/crypto.rs` and new `tests/conformance/core/crypto-*.nvst` cases. No
`registry.rs` edit and no `lib.rs` edit: both rows go on `CLASS`
(`crates/nvs-stdlib/src/crypto.rs:630`), which is registered already. `KEY_PAIR`
(`crates/nvs-stdlib/src/crypto.rs:1044`) is the worked example for a row that answers an instance,
and `nvs_core_crypto_key_pair_read` (`:3039`) for building one.

- [ ] **`Crypto::generateKeyPair(Crypto\KeyKind $kind): Crypto\KeyPair`** — three kinds and never
      RSA: both RSA cases are a `LogicError` naming `KeyPair::read`, because `ring` generates none.
      The scalar is drawn through `crate::random::draw`
      (`crates/nvs-stdlib/src/random.rs:483`) rather than any crate's own generator, so a seeded
      test reproduces the key. **Decide there how a PKCS#8 is written**: the two 25519 kinds are a
      16-octet constant prefix in front of the scalar, which is exactly what `x25519_scalar`
      (`crates/nvs-stdlib/src/crypto.rs:2522`) reads back, and P-256 is either `p256`'s
      `EncodePrivateKey` — check the `pkcs8` feature carries the encoder before designing around it
      — or the same prefix over its SEC1 body. Build the instance as
      `nvs_core_crypto_key_pair_read` (`:3039`) does, and make the answer a key `read` accepts.
      `rule:core-classes/crypto-interop-tier`, `rule:core-api/shape-rules`.
- [ ] **`Crypto::agree(Crypto\KeyPair $mine, Crypto\PublicKey $theirs): secret bytes`** — over
      `agree_x25519` (`crates/nvs-stdlib/src/crypto.rs:1503`) and `agree_p256` (`:1539`), with
      `pair_of` (`:3008`) and `key_of` (`:2865`) as the two readers. Two different curves is a
      `LogicError`, as is a kind that agrees nothing (Ed25519, either RSA); a non-contributory
      X25519 secret is a `RuntimeError`, being a verdict on the point a peer sent. The frozen set's
      `/ecdh/vectors` carries both curves' pairs and the secret they agree on, which is what a case
      asserts against. `rule:core-classes/crypto-interop-tier`.

## Backlog

- `examples/webcrypto.nvs` — the goal's acceptance fixture, which no stage has written yet; the
  driver reports it missing after every session. `docs/agent/loop-goal.md`.
- The standing decisions say PEM is read "with what the graph already carries
  (`crates/nvs-host/src/tls.rs:121`)"; that reader is `rustls`'s and `nvs-stdlib` cannot reach it —
  `pem-rfc7468` is not in the lockfile, so `der`'s own is off too. `pkcs8_der` decodes the armour
  itself. Worth correcting in the record when stage 4 is recorded.
- `Core\Jwe` and `Jwt::verifyIssued` are still unregistered in
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`; both are later stages of this
  goal.
