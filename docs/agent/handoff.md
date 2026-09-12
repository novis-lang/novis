# Handoff

## State

**Goal `webcrypto` — `Core\Jwe` is now held to the frozen set from both sides.** Every `jwe` vector in
`crates/nvs-stdlib/tests/vectors/webcrypto.json` is written again, byte for byte, by a new
`#[cfg(test)] mod tests` at the foot of `crates/nvs-stdlib/src/jwe.rs`: the three `dir` tokens, the PBES2
one and both `ECDH-ES` ones, each out of the vector's own randomness. The three refusals no `.nvst` case
reaches — *an epk off the P-256 curve*, *an X25519 epk of low order*, *p2s one octet short* — are asserted
at `content_key` answering `None`, so the test names the branch rather than the shared sentence.

**`encrypt` draws, and nothing below it does.** `nvs_core_jwe_encrypt` keeps every
`crate::random::draw`; the key management and the AEAD are four seams beside `compact` that take what
was drawn — `direct`, `wrapped`, `ephemeral` and `sealed`. No behaviour changed, and the six `.nvst`
cases are unchanged and green.

**The last handoff's stage numbers were wrong, and this one corrects them.**
`docs/agent/loop-goal.md`'s own `## Stage` headings put JWS — `Jwt::sign` over a pair, `Jwt\KeySet`,
`verifyIssued<T>` — at **stage 6**, and the vector replay plus `examples/webcrypto.nvs` at **stage 7**.
What landed here is stage 7's `jwe` half; stage 6 is untouched, and the failing acceptance check
(`examples/webcrypto.nvs`) stays red until `Jwt::verifyIssued` exists.

**Decided, with the token in front of it:** `encrypt` writes no `kid`.
`rule:security/jwe-compact-subset` gives `Jwe\Key`'s constructors one parameter each and none of them is
a key name, so the set's *dir with a kid* vector is decrypt-only; what the Rust test holds for it is its
header minus that one member.

## Next group

**Stage 6: JWS — `Core\Jwt` over a key pair** — one file set: `crates/nvs-stdlib/src/jwt.rs`,
`crates/nvs-stdlib/src/registry.rs`, and for the last item `crates/nvs-types/src/expr/args.rs`. The
goal's § *Standing decisions* fixes the whole surface; `docs/agent/loop-goal.md:173` is the stage.

- [ ] **`Jwt::sign` takes a `Crypto\KeyPair` beside a shared key, with the trailing
      `{kid?, typ?, embedKey?}` bag.** `rule:security/algorithm-comes-from-the-key`: the pair's kind is
      ES256, EdDSA, RS256 or PS256 and there is no `alg` argument. The five edits sit at
      `crates/nvs-stdlib/src/jwt.rs:453` (`nvs_core_jwt_sign`) and the row and card above it; the HS256
      path, the canonical header and the payload layout do not change, so every existing case passes
      untouched. Stage 0's two module-doc sections, `crates/nvs-stdlib/src/jwt.rs:11-28` and `:66-99`,
      are rewritten whole in this slice.
- [ ] **`Jwt\KeySet::read`, a second registered class in the same module.** Its admission rules are the
      goal's § *Standing decisions*; it joins the class list at `crates/nvs-stdlib/src/registry.rs:1748`
      the way `crate::jwe::KEY` does at `:1763`, and `rule:security/algorithm-comes-from-the-key` is why
      a `kid` is a lookup and never a try.
- [ ] **`Jwt::verifyIssued<T>`, beside `crates/nvs-stdlib/src/jwt.rs:523` (`nvs_core_jwt_verify`).**
      Its type argument joins the decode-site roster at `crates/nvs-types/src/expr/args.rs:1526`, which
      is `rule:security/derived-codec-qualifiers` and the one edit this goal makes outside
      `nvs-stdlib`; the claims come back `tainted` (`rule:security/verification-does-not-launder`) and
      `exp` stays mandatory (`rule:security/jwt-expiry-is-mandatory`).

## Backlog

- Stage 7's `jws` and `signatures` replay, once stage 6 lands: the shape is `jwe.rs`'s new
  `#[cfg(test)] mod tests`, deterministic algorithms byte for byte and ES256/PS256 held to verification
  (`docs/agent/loop-goal.md:218`).
- `examples/webcrypto.nvs` is stage 7's tail and the failing acceptance check
  (`docs/agent/loop-goal.md:222`).
- Stage 8 flips `core-classes/crypto-interop-tier`, `security/jwe-compact-subset` and
  `security/jws-issued-subset` to `shipped` (`docs/agent/loop-goal.md:230`).
- `crates/nvs-stdlib/src/crypto.rs` is past 4,800 lines with a third of it under `#[cfg(test)]` — a split
  is not this goal's (`docs/agent/playbook.md` § *Splitting a file that got too big*).
- The pack prints the goal's § *Standing decisions* but not its `## Stage` headings, and `[context]` has
  no field for goal prose: `python tools/peek.py 'docs/agent/loop-goal.md:re:^## Stage:12'` is the one
  call that says which stage the work is in.
