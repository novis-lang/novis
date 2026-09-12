# Handoff

## State

**Goal `webcrypto` — stage 4 is closed, members and prose alike.** `Core\Crypto`
(`crates/nvs-stdlib/src/crypto.rs:629`) carries the ten members of the goal's surface table below `Jwe`
and `Jwt`, and the prose over them now describes that surface rather than the AEAD-only class it was:
the module doc's four stage-0 sections are rewritten whole, `docs/reference/core/Crypto.md` covers
sealing, the two derivations, key pairs, agreement and signatures in three examples the generator runs,
and `docs/novis.md` is regenerated from it. `docs/spec/02-php-migration.md`'s two derivation rows were
already landed by `d56a555dc`; what was still false there was the section's own sentence about what
`Core\Crypto` is, and that is now the whole surface.

**Nothing in stage 4 is open.** The `expect(dead_code)` markers left in `crypto.rs` are `wrap_key`,
`unwrap_key`, `concat_kdf` and the two key-wrap constants at `:413-419` — stage 5's, and their `reason`
strings still say "stage 4 registers the members", which stage 5 rewrites as it deletes them.

**The acceptance check the driver reports is stage 7's, not a regression.** `examples/webcrypto.nvs`
opens one of the frozen set's JWE tokens and verifies one of its ID tokens, so it cannot be written
before `Core\Jwe` and `Jwt::verifyIssued` exist (`docs/agent/loop-goal.md:222`).

## Next group

**Stage 5: `Core\Jwe`, compact, `A256GCM` alone** — one file set: a new `crates/nvs-stdlib/src/jwe.rs`
over `crypto.rs`'s crate-private primitives, plus its two registration sites. The subset is the goal's
§ *Standing decisions*, *JWE, the subset* and *JWE answers like the rest of the roster*;
`rule:security/algorithm-comes-from-the-key` is what picks `alg` from the key's type.

- [ ] **Decide the key parameter's spelling first, in one read** — `CoreTy::Union`
      (`crates/nvs-stdlib/src/registry.rs:517`) exists; whether a union admits `SecretBlob` and
      `SecretText` members is the one thing the goal left open, and its named fallback is a `Jwe\Key`
      class with one static per kind. Whichever holds, the class is still two members.
- [ ] **`crates/nvs-stdlib/src/jwe.rs`, new, with its module doc and the three key-management paths**
      — `dir` over `gcm_seal_under` (`crates/nvs-stdlib/src/crypto.rs:1541`), `ECDH-ES` over
      `agree_p256`/`agree_x25519` and `concat_kdf` (`:2455`), `PBES2-HS256+A128KW` over `derive_key`
      and `wrap_key` (`:2394`), never a second copy of any of them. Registered beside
      `crate::jwt::CLASS` (`crates/nvs-stdlib/src/registry.rs:1748`) and in `lib.rs`.
- [ ] **`Jwe::encrypt`, the five edits** (`docs/agent/conventions.md` § *A `Core` member*) — the header
      written canonically, members sorted and no whitespace, which is what lets stage 7 hold it to the
      frozen set byte for byte; `crates/nvs-stdlib/src/jwt.rs:364` is the same writing over a JWS.
      `rule:security/protocol-roster`.
- [ ] **`Jwe::decrypt`, the five edits** — the header's allowed parameters are `alg`, `enc`, `epk`,
      `p2s`, `p2c`, `kid`, `typ` and `cty` and every other one is refused, the protected header is
      length-capped before it is parsed, the key ring is tried in order as
      `crates/nvs-stdlib/src/signature.rs:48`'s is, and the payload comes back `tainted string`. The
      compact split to mirror is `crates/nvs-stdlib/src/jwt.rs:528`.
      `rule:security/verification-throws-and-compares-in-constant-time`.

## Backlog

- `examples/webcrypto.nvs` — the acceptance fixture, still unwritten; it is stage 7's.
- `crypto.rs:413-419`'s two `expect(dead_code)` reasons name stage 4; stage 5 deletes them.
- Standard Webhooks' `v1a` and RFC 9421 signatures are package code over `sign`/`verify`, not `Core`
  (goal § *Standing decisions*, *Raw signatures*).
- `Jwt::sign` over a key pair and `Jwt\KeySet` are stage 6's, over these two members' primitives.
