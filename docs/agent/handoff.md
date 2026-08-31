# Handoff

## State

**Stage 4 is closed: ADR 0060 § 1's four-entry roster is complete.** `Core\Jwt` is rows, cards,
bodies and three `.nvst` cases in `crates/nvs-stdlib/src/jwt.rs`, beside `signed_cookie.rs`,
`csrf.rs` and `totp.rs`. Both of stage 4's named checks are green, including the `-p nvs-types` one
the item did not mention (see the playbook bullet this session added).

**`Core\Jwt` makes ADR 0060 § 4's three rules properties of a signature rather than checks in a
body**, and `jwt.rs`'s module doc is the one home for all of it: there is one algorithm so `alg` has
nothing to select from, the lifetime is a positional `Duration` and `exp`/`iat` in `$claims` are a
`LogicError` so expiry cannot be left out, and verification throws rather than answering a falsy
value. Every pre-signature failure is one sentence; expiry is the one refusal with its own, and it is
safe because it is reached only after the signature has been believed.

**`CoreTy::TaintedStr` is new, and it is the return-position spelling ADR 0060 § 5 needed.** A `Qual`
says what a member does with an *argument*, so the strongest thing it could promise was
`Contagious`'s conditional; § 5 wants the claims `tainted` whatever the token was. It lowers in
`crates/nvs-types/src/core_lib.rs:398` and spells in `crates/nvs-cli/src/meta.rs:299`; every other
walk over `CoreTy` ends in a wildcard, so nothing else moved.

**What that spends is structured claims.** `nvs_types` has no tainted array, so `verify` answers
`array<tainted string>` and a claim whose JSON value is `null`, an object or an array is **refused**
(ADR 0095) rather than flattened to a text spelling nothing else reads back. Tokens from an issuer
that nests claims will not verify. The widening, if it matters, is `Core\Json::decodeAs<T>` carrying
`tainted` into a declared shape — a `nvs_types` question, and `jwt.rs`'s module doc records it.

**The driver's acceptance failure is still stage 5's, not a regression** — `examples/http.nvs` names
`Core\Http::allowUrl`, which no stage before 5 lands, and a non-`0` stage's cargo checks run after
the program legs, so that fixture now masks a stage-4 list that is entirely green. Closing it is the
next group.

**Manifest gaps.** Two dead `[context] modules` selectors (`crates/nvs-host/src/pool.rs`,
`crates/nvs-host/src/stream.rs`); it wants `crates/nvs-runtime/src/commands.rs`,
`crates/nvs-types/src/defaults.rs`, `crates/nvs-test/src/case.rs`, `crates/nvs-cli/src/main.rs`,
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-stdlib/src/crypto.rs`, `crates/nvs-stdlib/src/jwt.rs`
and `crates/nvs-types/src/core_lib.rs` added. **`[context] adrs` still needs ADR 0060 §§ 1, 4, 5** —
none is printed, all three specify this roster, and this session paid for all three. Add ADR 0058
§§ 1-2, 4-5 and ADR 0074 §§ 5-7 for the group below. Nothing is blocked.

## Next group

**Stage 5's outbound half, which is what the acceptance run is stuck on.** The file set is the one
every roster entry has used: a new module beside `crates/nvs-stdlib/src/jwt.rs`,
`crates/nvs-stdlib/src/registry.rs:1209` (the `CLASSES` tail), `crates/nvs-stdlib/src/lib.rs:233`
and `:338` (the `mod` line and the `address` chain), and
`crates/nvs-types/src/core_lib.rs:729` (the tests module that already holds this goal's
signature-shape assertions).

- [ ] **The compile-time half first, because it decides the rows** — ADR 0058 §§ 1-2 and ADR 0074
      §§ 5, 7. An outbound URL parameter is a `Qual::Sink` so a `tainted` operand is a diagnostic,
      `allowUrl` is a `Qual::Launder` that pins, no member spells an unbounded timeout, and a `POST`
      retried without an idempotency key is a compile error. The four named checks are
      `an_outbound_url_parameter_refuses_a_tainted_operand`, `allow_url_pins_what_it_launders`,
      `no_client_member_accepts_an_unbounded_timeout` and
      `a_post_retried_without_an_idempotency_key_is_a_compile_error`, all in
      `crates/nvs-types/src/core_lib.rs:729`. `Qual::Launder` beside `Qual::Sink` on one class is the
      shape `crates/nvs-stdlib/src/signed_cookie.rs:136` already carries.
- [ ] **`Core\Http`'s rows, cards and bodies**, over `crates/nvs-stdlib/src/registry.rs:1209` and
      `crates/nvs-stdlib/src/lib.rs:338`. `examples/http.nvs` wants `status=200`, `body=ok`,
      `refused: tainted url`, `refused: denied range` and `deadline hit`, which is the acceptance
      check that has been red since session 0002.
- [ ] **The address policy lives in the capability, not in the client** — ADR 0058 §§ 4-5, over
      `crates/nvs-config/src/capability.rs:189` (`Capabilities::allows`, the one predicate a door
      asks) and `crates/nvs-config/src/capability.rs:58` (`Scope`, which is what an address range
      would have to become). The named checks are
      `the_address_policy_is_read_from_the_capability_and_not_from_the_client`,
      `a_denied_address_range_fails_before_a_connection_is_made` and
      `a_redirect_is_re_checked_against_the_same_policy`.

## Backlog

- Base32 in `Core\Encoding`, which `Core\Totp` needs for an `otpauth://` URI and nothing spells —
  RFC 4648 § 6, at `crates/nvs-stdlib/src/encoding.rs:330` and `:637`.
- ADR 0074 § 6's jittered retry under one covering deadline, and § 2's `traceparent` — the two
  stage-5 checks the group above does not reach.
- `nvs_types` has no `tainted array<T>`, so a member cannot answer a structured value that keeps
  ADR 0024's qualifier; `crates/nvs-stdlib/src/jwt.rs`'s module doc is where the need is recorded.
- Item 35's `property<T>` still holds this goal's one ADR slot, per `docs/agent/loop-goal.md`.
- Goal 5 needs a reachable Docker daemon for ADR 0067's five-driver matrix; the driver preflights it.
