# Handoff

## State

**Goal `webcrypto` — stage 4's member half is on disk.** `Core\Crypto` holds five rows
(`crates/nvs-stdlib/src/crypto.rs:402`): `generateKey`, `seal`, `open`, and now `deriveKey` and
`expandKey` over the crate-private `derive_key`/`expand_key`, which have dropped their
`expect(dead_code)` markers along with `DERIVED_LEN`, the two iteration bounds, `MIN_SALT_LEN` and
`pbkdf2_sha256`. `derive_key` takes its count as a `u64` so a number past the ceiling is reported as
the number that was written rather than wrapped into a plausible one.

**A row can now spell `secret string`**: `CoreTy::SecretText(Qual)`
(`crates/nvs-stdlib/src/registry.rs:319`), `SecretBlob`'s twin on the text base, interned as
`Ty::SecretString` by `crates/nvs-types/src/core_lib.rs:431`. It exists because `deriveKey`'s
password is `secret` and nothing could write one: `CoreTy::Text` refuses a `secret` argument, and
`Qual::Reveal` — the only mark that admits one — is closed to `Core\Secret` and `Core\Password`
because it *removes* the qualifier.

Stage 3's one unbuilt row is still the JWK thumbprint. `examples/webcrypto.nvs`, which the driver's
acceptance check names, is stage 7's fixture and is written last. Nothing is blocked.

## Next group

**Stage 4: the key kinds and the two key classes** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, `crates/nvs-stdlib/src/registry.rs` and new
`tests/conformance/core/crypto-*.nvst` cases. The goal's § *Standing decisions* fixes every signature;
`docs/agent/conventions.md` § *A `Core` member* is the five edits, and `deriveKey`'s row is the worked
example of a required argument with `defaults: &[]`.

- [ ] **`Crypto\KeyKind` becomes an enum** beside `CIPHER` at `crates/nvs-stdlib/src/crypto.rs:366`,
      registered in `ENUMS` at `crates/nvs-stdlib/src/registry.rs:2522` with its `EnumDoc` — `P256`,
      `X25519`, `Ed25519`, `RsaPkcs1` and `RsaPss`, no default. It maps onto `SignatureKind` at
      `crates/nvs-stdlib/src/crypto.rs:1241`, which already holds the four signature schemes, and the
      two RSA cases are `rule:security/algorithm-comes-from-the-key` held for the one key type two
      JWS algorithms share.
- [ ] **`Crypto\PublicKey` and `Crypto\KeyPair` become classes** in `crypto.rs`, over
      `read_p256_point` at `crates/nvs-stdlib/src/crypto.rs:1016` and `read_signing_key` at
      `crates/nvs-stdlib/src/crypto.rs:1285`. `Core\Hash\Stream` at
      `crates/nvs-stdlib/src/hash.rs:447` is the worked example of a class with `instance:` members
      and `slots:`; `rule:core-api/shape-rules` R14 is why a key is an object at all. Every public key
      is validated at `read` — the goal's § *Standing decisions* fixes which failure is a
      `RuntimeError` and which a `LogicError`.
- [ ] **`generateKeyPair` and `agree` become rows**, over `agree_x25519` at
      `crates/nvs-stdlib/src/crypto.rs:994` and `agree_p256` at `crates/nvs-stdlib/src/crypto.rs:1031`,
      both of which already refuse the all-zero shared secret. `generateKeyPair` refuses both RSA
      kinds with a `LogicError` naming `KeyPair::read`, and `agree` answers a coordinate meant for
      `expandKey` — `rule:security/secret-qualifier` is what the answer's type is held to.

## Backlog

- `Crypto::sign`/`verify` rows over `sign` (`crates/nvs-stdlib/src/crypto.rs:1326`) and
  `verify_signature` (`:1205`) — stage 4's last pair, after the key classes.
- The JWK thumbprint, stage 3's one unbuilt row — no function, no test (docs/agent/loop-goal.md § Stage 3).
- `examples/webcrypto.nvs`, the goal's stage 7 `exact` fixture (docs/agent/loop-goal.md § Stage 7).
- No reject case pins the *admission* side of `CoreTy::SecretText`: that a `secret string` reaches
  `deriveKey` where a plain `string` parameter would refuse it (`rule:security/secret-qualifier`).
- `docs/reference/core/Crypto.md` owes a third section when the key classes land.
