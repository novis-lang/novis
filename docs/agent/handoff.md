# Handoff

## State

**Goal `webcrypto` — Novis reads what a browser encrypts and what an issuer signs. Stage 2 is
landed; no Rust has been written yet.** Goal `template-format`'s whole list is this goal's Stage 1
floor and is green.

**[ADR 0179](../decisions/0179.md) is the design, accepted, and it is the one home for it** — the
goal's § *Standing decisions* is now transcribed and argued there, with every open question closed.
It creates `rule:core-classes/crypto-interop-tier`, `rule:security/jwe-compact-subset` and
`rule:security/jws-issued-subset`, all `designed`, and amends `rule:security/protocol-roster`, whose
roster now names `Core\Jwe`. `docs/spec/01-core-library.md` § 16's two rows carry the same surface.

**The two questions the record was asked to settle, both answered from the checker rather than
assumed — do not re-derive them, § *Investigation* carries the `file:line` for each.** A union may
carry a `secret bytes` arm and it works, so `Jwt::sign`'s `secret bytes|Crypto\KeyPair` key is an
ordinary row; a union may **not** carry a `secret string` arm, because `CoreTy` has no such atom and
the `Qual::Reveal` mark that accepts one is closed to `Core\Secret` and `Core\Password` and is
unavailable inside a union regardless. **So JWE's key takes the reserved fallback**: `Jwe\Key` with
four statics — `shared`, `password`, `recipient`, `own` — and `Jwe` stays at two members. And
structured claims are **a second member, `Jwt::signObject`**, not a union arm: a `CoreTy::Shape` may
never be a union member and there is no `object` spelling, and the split lets that member's key be a
`Crypto\KeyPair` alone, which turns a runtime `LogicError` into a compile-time mismatch.

## Next group

**Stage 3: the primitives, under their published vectors** — one file set: `Cargo.toml`,
`crates/nvs-stdlib/Cargo.toml`, `crates/nvs-stdlib/src/crypto.rs`, `THIRD-PARTY-LICENSES.txt`.

- [ ] **The dependencies land, and `ring` reaches `nvs-stdlib` directly** — `aes-gcm` 0.11, `aes-kw`
      0.3, `p256` 0.14 with `ecdh`, `x25519-dalek` 3.0, `hkdf` 0.13, `pbkdf2` 0.13, default features
      off and no `getrandom`, pinned in the workspace manifest's `[workspace.dependencies]` block and
      named at `crates/nvs-stdlib/Cargo.toml:11`; `ring` at the version the lockfile already
      resolves. Regenerate `THIRD-PARTY-LICENSES.txt`
      in the same commit (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`),
      and widen `ring`'s recorded answer from TLS to JWS under
      `rule:packaging/a-c-dependency-answers-two-questions` — ADR 0179 § 8 is the argument, already
      made.
- [ ] **AES-256-GCM, crate-private, against NIST's 256-bit-key GCM vectors plus a flipped tag
      refused** — beside `seal_under` at `crates/nvs-stdlib/src/crypto.rs:342` and `open_under` at
      `:399`, taking its nonce as an argument at the crate-private level so stage 7 can replay
      WebCrypto's inputs. Layout `nonce(12) ‖ ciphertext ‖ tag(16)`, per
      `rule:core-classes/crypto-interop-tier`. No `Core` member is registered yet — that is stage 4.
- [ ] **PBKDF2-HMAC-SHA256 and HKDF-SHA256, with the bounds checked before the first HMAC** — RFC
      7914 § 11 and RFC 5869 Appendix A.1–A.3 as test source with the section named beside each,
      appended at `crates/nvs-stdlib/src/crypto.rs:559`. Iterations refused below 100,000 and above
      2,000,000 and a salt below 16 octets, which is `rule:security/jwe-compact-subset`'s PBES2 bound
      written once here rather than twice.

## Backlog

- Stage 3's remaining rows — X25519, ECDH P-256, AES Key Wrap, Concat KDF, the four signature
  algorithms, the JWK thumbprint — `docs/agent/loop-goal.md` § *Stage 3* has the vector per row.
- `ring` may take no randomness from its caller for ES256/PS256; stage 3 confirms it and says so in
  the module doc (`docs/agent/loop-goal.md:136`, marked not checked).
- Structured claims under a *shared* key stay out of this goal — ADR 0179 § *Consequences*.
- JWE-encrypted ID tokens and RSA-OAEP with them, skipped by the user — ADR 0179 § *Alternatives
  rejected*.
- `Core\Signature` is still the roster's one entry with no member, module or case —
  `rule:core-classes/signature`.
