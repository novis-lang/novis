# Handoff

## State

**Goal `webcrypto`: stage 6 (JWS) is complete** — the check's `cases` list is now sixteen paths that
all exist and pass, paired against the `jwt-` cases on disk by their `--TEST--` lines the way stage
5's was, with a comment per group naming the claim that group holds.

Eight of the ten claims the check named were already on disk under names that do not say so:
`embedKey`'s proof of possession is inside
`tests/conformance/core/jwt-sign-writes-the-bag-into-a-sorted-header-and-nothing-else.nvst`, the
`rsaScheme` option is inside the two `jwt-a-key-set-*` cases, and the unqualified written type is
refused by `tests/conformance/reject/verify-issued-holds-its-written-type-to-the-tainted-qualifier.nvst`.

Two were genuinely unheld and were written this session: `verifyIssued` refusing `alg: none`, a token
MAC'd under the key the issuer publishes and an `alg` the key does not carry — four attacks, one
sentence — and a list claim and an object claim coming back as the written type, asked in both
spellings and agreeing with `Core\Json::decodeAs` over the same payload text.

`python tools/verify.py` stopped at `test` on one failure,
`crates/nvs-server/src/serve.rs:7958`'s fleet-ceiling test — the load-dependent one earlier sessions
already saw, which passes under `cargo test -p nvs-server the_in_flight_ceiling_is_fleet_wide` on its
own. The five steps before it were clean; the `.nvst` trees and clippy did not run, because the gate
stops at the first failure and this session changed no Rust. The sixteen cases the stage 6 check
names were run directly and all pass.

## Next group

**Stage 7: WebCrypto both directions and the example, whose `cargo-named` check is in exactly the
state stage 6's was** — one file set: the two goal files' stage 7 block and the `webcrypto_*` tests
in `crates/nvs-stdlib/src/{jwe,jwt,crypto}.rs`. Seventeen such functions are on disk and the check
names fourteen, one of which was split in three. The goal's § *Standing decisions* owns the frozen
vector set and the roster the refusal vectors are judged against.

- [ ] **Pair stage 7's fourteen `tests` names against the `webcrypto_*` functions on disk**, and
      rewrite the list in place: the list is `docs/agent/loop-goal.toml:9186` and its copy
      `docs/agent/goals/47-webcrypto.toml:9162`, and
      `webcrypto_jwe_vectors_reencrypt_to_the_same_token_from_the_same_randomness` is three functions
      at `crates/nvs-stdlib/src/jwe.rs:1304`, `crates/nvs-stdlib/src/jwe.rs:1355` and
      `crates/nvs-stdlib/src/jwe.rs:1404`.
- [ ] **Name the JWS test the list does not** — `crates/nvs-stdlib/src/jwt.rs:2998` is
      `webcrypto_jws_signs_reproduce_the_randomized_token_as_far_as_it_is_fixed`, the other half of
      the deterministic-token claim at `crates/nvs-stdlib/src/jwt.rs:2940`, and the goal's § *Standing
      decisions* says an ES256 or PS256 signature is kept from the file on disk while it still
      verifies.
- [ ] **Run what the two stage 7 checks run** — `cargo test -p nvs-stdlib webcrypto_` for the named
      tests, and `examples/webcrypto.nvs` for the seven lines the `exact` check wants at
      `docs/agent/loop-goal.toml:9207`; whatever fails is the rest of this group.

## Backlog

- A shape-typed field decodes in no codec — `crates/nvs-stdlib/src/json.rs`'s own known gap, which is
  why a nested claim is written as a deriving class.
- Stage 8's rulebook checks (`tools/rules.py --show core-classes/crypto-interop-tier` says `shipped`)
  are untouched — `docs/agent/loop-goal.toml:9221`.
- `Jwt::verify`'s HS256 path has no case pairing it with `verifyIssued` over one token, and nothing
  asks for one — `docs/agent/loop-goal.md` § *Standing decisions*, JWS subset.
