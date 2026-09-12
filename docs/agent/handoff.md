# Handoff

## State

**Goal `webcrypto` — stage 4's member half and all three of its closed enums are on disk.**
`Core\Crypto` holds five rows (`crates/nvs-stdlib/src/crypto.rs:534`), and `ENUMS` now registers
`Crypto\Cipher` beside `Crypto\KeyKind` (`crates/nvs-stdlib/src/crypto.rs:427`) and
`Crypto\KeyFormat` (`crates/nvs-stdlib/src/crypto.rs:498`): five kinds, three formats, no default on
either, and each case's own written-out constant because a member reads the tag back out of an
argument slot.

**Nothing takes the two new enums yet**, which is the one thing `ENUMS`' own doc
(`crates/nvs-stdlib/src/registry.rs:2522`) asks a row not to be — a case a program can write and pass
nowhere. The group below closes it: `KeyFormat` at `PublicKey::read`/`write`, `KeyKind` at both key
classes and at `generateKeyPair`.

Stage 3's one unbuilt row is still the JWK thumbprint, now folded into the codec item below.
`examples/webcrypto.nvs`, which the driver's acceptance check names, is stage 7's fixture and is
written last. Nothing is blocked.

## Next group

**Stage 4: the two key classes** — one file set: `crates/nvs-stdlib/src/crypto.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/lib.rs` and new
`tests/conformance/core/crypto-*.nvst` cases. Two discoveries this session paid for, so the next does
not: a `Core` instance slot holds a Novis `Value` and never native state
(`crates/nvs-stdlib/src/instance.rs:15`), `Core\IO\File` (`crates/nvs-stdlib/src/io.rs:1330`) is the
current worked example of `slots:` beside `instance:`, `instance::build`
(`crates/nvs-stdlib/src/instance.rs:367`), `instance::receiver`
(`crates/nvs-stdlib/src/instance.rs:473`) and `instance::slot`
(`crates/nvs-stdlib/src/instance.rs:513`) are the three calls a member makes, the receiver is in
neither `names` nor `params` but *is* `args[0]` so `args: [N]` counts it, and a class costs one line
in `registry::CLASSES` plus one `.or_else` in `address_of` (`crates/nvs-stdlib/src/lib.rs:343`). And
the frozen set carries `pkcs8`, `pem`, `spki`, `jwk`, `jwkMinimal` and `thumbprint` for every JWS key
(`crates/nvs-stdlib/tests/vectors/webcrypto.json`, under `/jws/keys/*`), so every codec below is
pinned to WebCrypto's own bytes rather than to itself.

- [ ] **The public-key codec, crate-private and pinned to the set** — SPKI, raw and JWK in and out for
      the five kinds, beside `read_p256_point` at `crates/nvs-stdlib/src/crypto.rs:1152`, with RFC
      7638's thumbprint over the same members. The workspace manifest's `p256` row already turns on
      its `pkcs8` feature, so `p256::pkcs8::spki` reads and writes the outer `SubjectPublicKeyInfo` and only RSA's inner
      PKCS#1 `RSAPublicKey` is left to walk; the two 25519 SPKIs are a fixed prefix over 32 octets.
      `VerifyingKey` at `crates/nvs-stdlib/src/crypto.rs:1293` is the shape the components feed.
- [ ] **`Crypto\PublicKey` becomes a class** over that codec, beside `Core\Crypto`'s own roster at
      `crates/nvs-stdlib/src/crypto.rs:534` and shaped like `Core\IO\File` at
      `crates/nvs-stdlib/src/io.rs:1330` — `read`, `write`, `kind`, its slots holding the validated
      SPKI and the kind's tag so one canonical form is stored and every `write` is a re-encode. The
      goal's § *Standing decisions* fixes which failure is a `RuntimeError` (a key
      off the wire) and which a `LogicError` (`Raw` under an RSA kind, a JWK carrying `d`, a key the
      program itself built wrong); `rule:core-api/shape-rules` R14 is why a key is an object at all.
- [ ] **`Crypto\KeyPair` becomes a class** over `read_signing_key` at
      `crates/nvs-stdlib/src/crypto.rs:1421` — `read` taking PKCS#8 as DER or as one PEM `PRIVATE KEY`
      block and refusing PKCS#1 and encrypted PKCS#8, then `write` and `publicKey`.
- [ ] **`generateKeyPair` and `agree` become rows**, over `agree_x25519` at
      `crates/nvs-stdlib/src/crypto.rs:1130` and `agree_p256` at `crates/nvs-stdlib/src/crypto.rs:1167`,
      both of which already refuse the all-zero shared secret. `generateKeyPair` refuses both RSA kinds
      with a `LogicError` naming `KeyPair::read`, and `agree` answers a coordinate meant for
      `expandKey` — `rule:security/secret-qualifier` is what the answer's type is held to.

## Backlog

- `Crypto::sign`/`verify` rows over `sign` (`crates/nvs-stdlib/src/crypto.rs:1462`) and
  `verify_signature` (`crates/nvs-stdlib/src/crypto.rs:1341`) — stage 4's last pair.
- No `.nvst` case pins either new enum; the cases arrive with the members that take them, three per
  member (docs/agent/conventions.md § *A `Core` member*).
- `examples/webcrypto.nvs`, the goal's stage 7 `exact` fixture (docs/agent/loop-goal.md § Stage 7).
- No reject case pins the *admission* side of `CoreTy::SecretText`: that a `secret string` reaches
  `deriveKey` where a plain `string` parameter would refuse it (`rule:security/secret-qualifier`).
- `docs/reference/core/Crypto.md` owes a third section when the key classes land.
