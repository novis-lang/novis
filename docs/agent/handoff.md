# Handoff

## State

**Goal `webcrypto` — stage 4 has opened: `Core\Crypto::seal` and `::open` take a required
`Core\Crypto\Cipher`.** The enum is `crates/nvs-stdlib/src/crypto.rs:407` — `XChaCha20Poly1305` and
`Aes256Gcm`, no default — registered in `ENUMS` at `crates/nvs-stdlib/src/registry.rs:2515`, and `keyed`
splits on it, so `gcm_cipher`, `gcm_seal_under` and `gcm_open_under` are reachable from a member and have
dropped their `expect(dead_code)` markers. Every call site moved in the same commit: `examples/crypto.nvs`,
the six conformance cases (one renamed, as the goal's stage-4 check names it) and
`docs/reference/core/Crypto.md`'s example.

**Stage 3's one unbuilt row is still the JWK thumbprint** — no function, no test. Everything else stage 3
names is on disk and asserted against published vectors, still carrying
`#[cfg_attr(not(test), expect(dead_code, …))]` until its row lands. Nothing is blocked.

## Next group

**Stage 4: the two derivations take their rows** — one file set: `crates/nvs-stdlib/src/crypto.rs` and
two new `tests/conformance/core/crypto-*.nvst` cases. The five edits are
`docs/agent/conventions.md` § *A `Core` member*, and `seal`'s new row is the worked example for a
required enum argument with `defaults: &[]`.

- [ ] **`deriveKey` and `expandKey` become rows** — `crates/nvs-stdlib/src/crypto.rs:402`'s `methods`
      gains both before `instance: &[]` at `crates/nvs-stdlib/src/crypto.rs:447`, with cards beside
      `OPEN_DOC` and arms in `address` at `crates/nvs-stdlib/src/crypto.rs:543`. The bodies wrap
      `derive_key` at `crates/nvs-stdlib/src/crypto.rs:793` and `expand_key` at
      `crates/nvs-stdlib/src/crypto.rs:834`, both of which already hold the bounds and answer
      `CoreTy::SecretBytes`; the `expect(dead_code)` on each is what deletes itself.
      `rule:security/secret-qualifier` is what the answers' type is held to, and the goal's
      § *Standing decisions* fixes that the iteration count is required and unbounded by no default.
- [ ] **`crypto-pbkdf2-refuses-an-iteration-count-or-salt-outside-its-bounds.nvst`** — the bound named on
      both sides at each edge: 99,999 refused beside 100,000 accepted, 2,000,001 beside 2,000,000, and a
      15-octet salt beside a 16-octet one, each a `LogicError` because the program chose the number.
      `crates/nvs-stdlib/src/crypto.rs:793` raises them; `rule:core-api/failure-throws` is the shape.
- [ ] **`crypto-derived-and-agreed-keys-are-secret-bytes.nvst`** — one question asked of `generateKey`,
      `deriveKey` and `expandKey`: each answer keys a `seal` and none of them assigns to a plain `bytes`.
      `rule:security/secret-qualifier` is the rule; `crates/nvs-stdlib/src/crypto.rs:402` is the roster it
      is asked of. The agreement half waits on `generateKeyPair`, so this case takes the derivations now
      and the name still fits.

## Backlog

- The JWK thumbprint, stage 3's unbuilt row — `docs/agent/goals/47-webcrypto.md` § stage 3.
- `Crypto\KeyKind`, `Crypto\KeyFormat` and the two key classes — the same five edits, stage 4's larger half.
- `Core\Crypto::sign` and `::verify` over the four signature algorithms already on disk — stage 4.
- `examples/webcrypto.nvs`, the goal's one missing fixture — stage 7, and the acceptance check that fails today.
