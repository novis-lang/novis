# Handoff

## State

**Goal `webcrypto`, stage 7 is complete.** `examples/webcrypto.nvs` prints the seven lines the
`exact` check at `docs/agent/loop-goal.toml:9162-9174` freezes, in order, at exit 0, and the
fourteen stage 7 test names are all green. Stage 8 — the rulebook — is what is left of the goal.

**The example signs the ID token it verifies, under the set's own issuer key.** The frozen set's ID
tokens all carry `exp` 1767229200, `verifyIssued` takes no clock at the Novis level and its `leeway`
is capped at five minutes (`crates/nvs-stdlib/src/jwt.rs:380-395`), so the goal's § *Stage 7* ask —
verify one of the set's ID tokens — is not reachable from a program running at the wall clock. What
the example does instead is read the set's `ed-1` pair from its frozen PKCS#8, publish the JWK
WebCrypto exported for it as a one-key JWKS, sign at this clock and verify through that set — then
swap `EdDSA` for `HS256` in the header of that same token and be refused. The JWE half does open one
of the set's own tokens.

**`Core\Jwe::decrypt` cannot be handed a token that arrived in a request.** Its `$token` is
`CoreTy::Text(Qual::Contagious)` over a `TaintedStr` return (`crates/nvs-stdlib/src/jwe.rs:185-194`),
and `admits_tainted_argument` (`crates/nvs-types/src/expr/quals.rs:299-308`) refuses a `tainted`
argument at a `Contagious` parameter whose answer is already `tainted`. `Core\Jwt::verify` and
`verifyIssued` are `Qual::Neutral` for exactly this reason and say so
(`crates/nvs-stdlib/src/jwt.rs:466-494`). That is the next group.

`nvs-host`'s two CPU-charging watchdog tests stay the known flake
(`crates/nvs-host/src/watchdog.rs:1051` and `:1103`): one fails per run under load, a different one
each time, and each passes alone.

## Next group

**Stage 7: the token a request hands in** — one file set: `crates/nvs-stdlib/src/jwe.rs`, with
`crates/nvs-types/src/expr/quals.rs:299` and `crates/nvs-stdlib/src/jwt.rs:466` read-only.
`rule:security/verification-does-not-launder`, and the goal's § *Standing decisions* paragraph
beginning "JWE answers like the rest of the roster".

- [ ] **Mark `Core\Jwe::decrypt`'s `$token` `Qual::Neutral`** at
      `crates/nvs-stdlib/src/jwe.rs:188`, rewriting the row's comment the way
      `crates/nvs-stdlib/src/jwt.rs:468-471` writes its own: the token is neutral on the way in
      because the payload is `tainted` on the way out whatever it was, so contagion has nothing
      left to carry and only refuses the ordinary call. Check whether `encrypt`'s payload
      (`crates/nvs-stdlib/src/jwe.rs:179`) is in the same position — its return is plain `Str`, so
      it is not — and whether `crates/nvs-types/src/core_lib.rs:940-990`'s qualifier audit counts
      the row.
- [ ] **Write the case that only compiles under it**:
      `tests/conformance/core/jwe-opens-a-token-that-arrived-tainted.nvst`, shaped like
      `tests/conformance/core/jwe-opens-every-token-webcrypto-sealed.nvst:1` — open one of the
      frozen `dir` tokens, hand the `tainted` payload back to `encrypt`, and open the `tainted`
      token that comes out. Add the Rust-side half beside the set's replays in
      `crates/nvs-stdlib/src/jwe.rs:1262` if the member's own tests do not already reach it.

## Backlog

- **Stage 8, the rulebook** — `rule:core-classes/crypto-interop-tier` and the goal's § *Stage 8*.
- **The set's ID tokens verify only at a clock nothing hands in** — a `.nvst` case cannot reach
  `verifyIssued` over a frozen token at all; `docs/agent/loop-goal.md` § *Stage 7*.
- What stays out of this goal is its own § *Standing decisions* last bullet, not this list.
