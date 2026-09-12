# Handoff

## State

**Goal `webcrypto` — stage 4's public-key codec is on disk; neither key class is.**
`crate::crypto` holds `PublicKey` (`crates/nvs-stdlib/src/crypto.rs:1389`) beside the `KeyKind`,
`KeyFormat` and `KeyRefusal` Rust enums (`:1258`, `:1321`, `:1359`) and the `impl` that reads and
writes all five kinds in all three encodings (`:1428`). Every check a public key gets is at the
read; the module doc's *one key, three encodings* section is the home of why, and
`rule:core-classes/crypto-interop-tier` is the rule. No new dependency — `p256`'s `pkcs8` feature
already carries `spki`, `der` and their `alloc`, so only RSA's wrapper is parsed and the three
curve kinds' `SubjectPublicKeyInfo` is a constant prefix over a fixed-width key.

**The design question stage 4 must answer, pre-authorized and not blocked.** A `Core` instance slot
holds a Novis `Value` and never native state (`crates/nvs-stdlib/src/instance.rs:15`). So
`Crypto\PublicKey`'s slots are its canonical SPKI plus its kind tag, and each member re-reads
through `PublicKey::read` — a DER parse and a point validation per call, on no hot path, never less
validated than the first read. `Crypto\KeyPair` cannot do the same *and* keep `SigningKey`'s doc
claim that a pair holds its parsed key across calls (`crates/nvs-stdlib/src/crypto.rs:2084`): item
2 below either takes `crate::regex`'s per-core cache, which `instance.rs:17` names as the blessed
answer, or rewrites that sentence. Nothing is blocked.

## Next group

**Stage 4: the two key classes** — one file set: `crates/nvs-stdlib/src/crypto.rs`,
`crates/nvs-stdlib/src/registry.rs` and new `tests/conformance/core/crypto-*.nvst` cases. Four
discoveries this session paid for, so the next does not. `crypto::address` is already in
`address_of`'s chain (`crates/nvs-stdlib/src/lib.rs:358`), so **a second class in this module costs
no `lib.rs` edit at all** — one line in `registry::CLASSES` beside `crate::io::FILE`
(`crates/nvs-stdlib/src/registry.rs:1428`) is the whole registration. `io::FILE`
(`crates/nvs-stdlib/src/io.rs:1330`) is the worked example of `methods:`/`instance:`/`slots:`
together, and `io::handle_of` (`crates/nvs-stdlib/src/io.rs:2201`) is the shape a slot reader
takes — `instance::receiver`, then `instance::slot`, then a `Fault::fatal` naming the slot.
`every_registry_rows_names_are_the_specs_signature_column` reads only spec §§ 1–12
(`crates/nvs-stdlib/tests/spec_registry_coverage.rs:694`), so § 16's prose row needs no signature
table. And the conformance floor is three cases per member, each asking a *different* question
(`crates/nvs-stdlib/tests/conformance_coverage.rs:137`), which three files calling all of one
class's members satisfy at once.

- [ ] **`Crypto\PublicKey` becomes a class** over the codec — `read` as a static, `write` and
      `kind` as instance members, slots holding the SPKI and the kind tag — beside `Core\Crypto`'s
      own roster at `crates/nvs-stdlib/src/crypto.rs:609`, with `PublicKey::read`
      (`crates/nvs-stdlib/src/crypto.rs:1428`) under it and `KeyRefusal::Bug` becoming the
      `LogicError`. `rule:core-classes/crypto-interop-tier`, `rule:core-api/shape-rules`.
- [ ] **`Crypto\KeyPair` becomes a class** over `read_signing_key`
      (`crates/nvs-stdlib/src/crypto.rs:2109`) — `read` and `write` over PKCS#8 as DER or one PEM
      block, `publicKey()` through the codec — and answers the `SigningKey` question in § State
      above. `rule:security/algorithm-comes-from-the-key`.
- [ ] **`generateKeyPair` and `agree` become rows** on `Core\Crypto`, over `agree_x25519`
      (`crates/nvs-stdlib/src/crypto.rs:1205`) and `agree_p256`
      (`crates/nvs-stdlib/src/crypto.rs:1242`), with `PublicKey::p256_point` as the point the
      second takes and both RSA kinds a `LogicError` at generation.
      `rule:core-classes/crypto-interop-tier`.

## Backlog

- `examples/webcrypto.nvs`, the driver's acceptance fixture, is stage 7's and is written last.
- Stage 3's JWK thumbprint is closed by this session; nothing of stage 3 is left open.
- JWE-encrypted ID tokens, RSA-OAEP with them, and key-set fetching stay out —
  `docs/agent/loop-goal.md` § *Not this goal*.
- `p256`'s curve arithmetic is unaudited and accepted; `Cargo.toml:583` is the home.
