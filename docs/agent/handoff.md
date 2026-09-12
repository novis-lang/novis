# Handoff

## State

**Goal `webcrypto`, stage 6 is open: `Core\Jwt::sign` now signs under a key pair.** Its key parameter is
the union `secret bytes|Crypto\KeyPair` (`crates/nvs-stdlib/src/jwt.rs:220`), which ADR 0179's
*Investigation* section already established is an ordinary row — the secret arm is written
`CoreTy::SecretBlob` and never the bare `CoreTy::SecretBytes`, because the registry's
unclassified-parameter audit walks a union's members. There is no `alg` argument anywhere:
`pair_alg` (`crates/nvs-stdlib/src/jwt.rs:168`) is a total function of the pair's kind — ES256, EdDSA,
RS256, PS256 — and an `X25519` pair is a `LogicError`.

**The header is assembled now, and sorted.** `header_of` (`crates/nvs-stdlib/src/jwt.rs:491`) writes
`{alg, jwk?, kid?, typ}` with no whitespace; `HEADER` stays as the hand-written spelling and
`the_default_header_is_the_written_constant` holds the two together, so the HS256 path is byte-identical
and every pre-existing `Jwt` case passes untouched. The bag is `{kid?, typ?, embedKey?}`
(`crates/nvs-stdlib/src/jwt.rs:224`); `embedKey` writes exactly what
`Crypto\PublicKey::write(KeyFormat::Jwk)` exports, and under a shared secret it is a `LogicError`.

**Claims did not change, and `verify` did not change.** `$claims` is still `array<string>` and
`Jwt::verify` is still HS256 over a shared key, so a pair-signed token is one it refuses with its one
sentence — a new conformance case counts that. Structured claims are a **second member**, `signObject`,
per ADR 0179 § 7, and are not on disk.

The failing acceptance check (`examples/webcrypto.nvs`) stays red until stage 7; nothing is blocked.

## Next group

**Stage 6: JWS — `Jwt\KeySet` and `verifyIssued`** — one file set: `crates/nvs-stdlib/src/jwt.rs`,
`crates/nvs-stdlib/src/registry.rs`, and for the last item `crates/nvs-types/src/expr/args.rs`. The
goal's § *Standing decisions* and ADR 0179 § 7 fix the whole surface.

- [ ] **`Jwt\KeySet::read(tainted string $jwks, {rsaScheme?})`, a second registered class in this
      module.** Its ten admission rules are the goal's § *Standing decisions*; it joins the class list at
      `crates/nvs-stdlib/src/registry.rs:1748` the way `crate::jwe::KEY` does at `:1763`, and its
      instance is built the way `Jwe\Key`'s is (`crates/nvs-stdlib/src/jwe.rs:851`, then
      `crate::instance::build`). `rule:security/algorithm-comes-from-the-key` is why a `kid` is a lookup
      and never a try. The class const goes beside `crates/nvs-stdlib/src/jwt.rs:248`.
- [ ] **`Jwt::signObject(object $claims, Duration $lifetime, Crypto\KeyPair $key, {kid?, typ?,
      embedKey?})`, beside `nvs_core_jwt_sign` at `crates/nvs-stdlib/src/jwt.rs:667`.** ADR 0179 § 7 is
      why it is a second member and not a union arm — a `CoreTy::Shape` may not be a union member. It
      shares `header_of`, `signer_at` and `embedded_jwk` and writes its subject as `Core\Json::encode`
      does, with `iat` then `exp` appended; its key is a pair **alone**, which is what makes structured
      claims under a shared key a type mismatch rather than a runtime refusal.
- [ ] **`Jwt::verifyIssued<T>`, beside `crates/nvs-stdlib/src/jwt.rs:785` (`nvs_core_jwt_verify`).** Its
      type argument joins the decode-site roster at `crates/nvs-types/src/expr/args.rs:1526`, which is
      `rule:security/derived-codec-qualifiers` and the one edit this goal makes outside `nvs-stdlib`;
      the claims come back `tainted` (`rule:security/verification-does-not-launder`) and `exp` stays
      mandatory (`rule:security/jwt-expiry-is-mandatory`).

## Backlog

- Stage 7's vector replay of `jws.signs` is what holds `sign`'s RSA rows byte for byte; the `.nvst` sweep
  covers the curve kinds only, because a pair of an RSA kind needs a PKCS#8 file to exist at all —
  `docs/agent/loop-goal.md:213`.
- `examples/webcrypto.nvs`, the acceptance fixture, is stage 7 — `docs/agent/loop-goal.md:191`.
- `Jwt::verify` still refuses a claim that is not text; a third-party token's structured claims are
  `verifyIssued<T>`'s answer and nothing widens `verify` — `crates/nvs-stdlib/src/jwt.rs:94`.
