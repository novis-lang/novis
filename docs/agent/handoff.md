# Handoff

## State

**Goal `webcrypto` — stage 5 is landed whole: `Core\Jwe` and `Core\Jwe\Key` are on disk, registered and
green.** `crates/nvs-stdlib/src/jwe.rs` holds the class, the four named constructors and the three
key-management paths over `crypto.rs`'s crate-private primitives; `rule:security/jwe-compact-subset` is
the home and the implementation matches it. Six `.nvst` cases are in `tests/conformance/core/`, and
`Core\Jwe` is struck from `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`.

**The frozen vector set already judges this, and it passes.** `jwe-opens-every-token-webcrypto-sealed.nvst`
decrypts all six `jwe.vectors` tokens in `crates/nvs-stdlib/tests/vectors/webcrypto.json` — `dir`
including the empty payload and the `kid` one, `ECDH-ES` over both curves, PBES2 at the floor — and
`jwe-refuses-every-unopenable-token-with-one-sentence.nvst` refuses thirteen of the twenty-three
`jwe.refusals` with one sentence. The `dir` header `encrypt` writes is byte-identical to the set's
(`eyJhbGciOiJkaXIiLCJlbmMiOiJBMjU2R0NNIn0`), asserted in
`jwe-round-trips-under-every-key-and-answers-a-tainted-payload.nvst`.

**`crypto.rs` opened five things to `Core\Jwe` and lost its five dead-code markers**, whose expectations
went unfulfilled the moment the caller landed: `agree`, `NoAgreement` and `generated_pkcs8` are
`pub(crate)` (the last taking a `who` for its two fatals), and `stored_key`/`stored_octets` are new — one
reader over both key classes, held to one layout by a `const` assertion beside the slot constants.

**Open, and named for stage 6:** `encrypt` writes no `kid`, so the set's *dir with a kid* vector is a
decrypt-only vector today. `rule:security/jwe-compact-subset` writes four constructors with one parameter
each and says nothing about a `kid`, so none was added; the session that writes the vector test decides
with the token in front of it.

## Next group

**Stage 6: the vector set as Rust tests, in the shape `crypto.rs`'s own `#[cfg(test)]` block gives them**
— one file set: a new `#[cfg(test)] mod tests` at the foot of `crates/nvs-stdlib/src/jwe.rs`, reading
`crates/nvs-stdlib/tests/vectors/webcrypto.json` the way `crates/nvs-stdlib/src/crypto.rs:4079`
(`the_derivation_ecdh_es_runs_matches_rfc_7518_appendix_c`) reads its own section.
`rule:security/jwe-compact-subset` is the rule; the `.nvst` cases already cover the member surface, so
what these add is what a case cannot reach — `encrypt` held to a frozen token byte for byte under the
vector's own randomness.

- [ ] **Split the AEAD and header halves out of `crates/nvs-stdlib/src/jwe.rs:728`
      (`nvs_core_jwe_encrypt`) so a test can seal with a given IV.** The member draws its IV and its
      PBES2 salt and content key through `crate::random::draw`
      (`crates/nvs-stdlib/src/random.rs:483`), which a test cannot hand fixed values to; a
      `fn sealed(cek, iv, protected, payload)` beside `crates/nvs-stdlib/src/jwe.rs:@compact` is the seam,
      and the member keeps the draw. No behaviour changes, so the six cases are the check.
- [ ] **`encrypt` reproduces the set's three `dir` tokens byte for byte.** Each vector carries
      `randomness.iv`; assert the whole token string against `token`, which is what makes the canonical
      header a property rather than a comment. Then PBES2, whose `randomness` is `cek`, `p2s` and `p2c`
      — `crates/nvs-stdlib/src/jwe.rs:@pbes2_key` is the derivation and
      `crates/nvs-stdlib/src/crypto.rs:2403` (`wrap_key`) the wrap.
- [ ] **The two `ECDH-ES` vectors, whose randomness is an ephemeral pair rather than a value.**
      `randomness.ephemeralPkcs8` is what `crates/nvs-stdlib/src/crypto.rs:2815` (`generated_pkcs8`)
      would have drawn, so the seam is the same split one level deeper. Then the `jwe.refusals` no case
      reaches — *an epk off the P-256 curve*, *an X25519 epk of low order*, *p2s one octet short* —
      asserted against `crates/nvs-stdlib/src/jwe.rs:1025` (`content_key`) answering `None` rather than
      against the sentence, so the test names the branch.

## Backlog

- `Jwe\Key` carries no `kid`, so `encrypt` writes none — decide it in stage 6 against the set's *dir with
  a kid* vector (`rule:security/jwe-compact-subset`).
- `examples/webcrypto.nvs` is stage 7 and still missing; it needs `Jwt::verifyIssued` as well as
  `Core\Jwe` (`docs/agent/loop-goal.md:222`).
- Stage 7's `Jwt::verifyIssued` and `Jwt\KeySet` are untouched; the goal's § *Standing decisions* holds
  the whole surface.
- `crates/nvs-stdlib/src/crypto.rs` is past 4,800 lines with a third of it under `#[cfg(test)]` — a split
  is not this goal's (`docs/agent/playbook.md` § *Splitting a file that got too big*).
