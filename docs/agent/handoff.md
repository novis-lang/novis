# Handoff

## State

**Stage 4's protocol half is open, and its first entry is closed.** `Core\SignedCookie::seal` and
`::open` are rows, cards and bodies in `crates/nvs-stdlib/src/signed_cookie.rs`, whose module doc is
the one home for which end of the key ring is the newest key, why `open` is the only verification in
the language that removes `tainted`, what a cookie is on the wire, and why two members sharing
`Core\Crypto`'s verb names is not ADR 0063 R17's "reachable two ways".

**There is one AEAD in `nvs-stdlib` and both classes are on the near side of it.**
`crates/nvs-stdlib/src/crypto.rs` now exposes `cipher` (306), `seal_under` (339) and `open_under`
(396) as `pub(crate)`, and `Core\Crypto`'s own two members are three lines each over them. That
refactor is the whole of why a signed cookie has no second nonce policy and no second opinion about
tags; `crypto.rs`'s module doc carries it, and `open_under`'s doc is the home of why "not authentic"
is one answer rather than three.

**`$keys[0]` is the newest key**, seals, and is the only key `seal` looks at; `open` walks the whole
ring in order. An empty ring and a ring entry that is not 32 octets are `LogicError`s naming the
index; everything else is one `RuntimeError` sentence. Three `.nvst` cases pin the order contract,
the four-way refusal agreement and the wire form's stated length.

**The driver's acceptance failure is stage 5's, not a regression.** `examples/http.nvs` names
`Core\Http::allowUrl`, which no stage before 5 lands, and a non-`0` stage's cargo checks run *after*
the program legs — so that fixture will mask stage 4's named checks until `Core\Http` exists. It is
the same shape `examples/crypto.nvs` had through sessions 0004-0009.

**Manifest gaps, unchanged plus one.** Two dead `[context] modules` selectors
(`crates/nvs-host/src/pool.rs`, `crates/nvs-host/src/stream.rs`); it wants
`crates/nvs-runtime/src/commands.rs`, `crates/nvs-types/src/defaults.rs`, `crates/nvs-test/src/case.rs`,
`crates/nvs-cli/src/main.rs`, `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-stdlib/src/password.rs`,
`crates/nvs-stdlib/src/crypto.rs` and now `crates/nvs-stdlib/src/signed_cookie.rs` added. **`[context]
adrs` still needs ADR 0060 §§ 1, 4, 5** — none was printed and all three specify the group below.
Nothing is blocked.

## Next group

**Stage 4's protocol half, entries two to four — ADR 0060 § 1's remaining bullets, each a named
check still owing.** They share one file set with what just landed:
`crates/nvs-stdlib/src/registry.rs:1158` (the `CLASSES` tail),
`crates/nvs-stdlib/src/lib.rs:248` and `:348` (the `mod` line and the `address` chain),
`crates/nvs-stdlib/src/crypto.rs:306` (the three shared helpers) and one new module each beside
`crates/nvs-stdlib/src/signed_cookie.rs:115`.

- [ ] **`Core\Csrf`, where the comparison is the only exposed operation** — ADR 0060 § 1's second
      bullet. **`Core\Session` is goal 6's**, so the session the token is bound to arrives as an
      argument rather than being read: a member answering a token and a member answering `bool`, and
      no member answering the expected token for a caller to `==`. Registered at
      `crates/nvs-stdlib/src/registry.rs:1158` and `crates/nvs-stdlib/src/lib.rs:348`, keyed through
      `crates/nvs-stdlib/src/crypto.rs:306`. The named check is
      `a_csrf_token_is_bound_to_the_session_that_issued_it`.
- [ ] **`Core\Totp`, with a bounded window and no replay** — ADR 0060 § 1's third bullet. There is no
      cross-request store in this goal (ADR 0059's is stage 6), so "no replay" has to be a *counter
      the member answers* for the caller to store, not state the class keeps. Same registration
      anchors: `crates/nvs-stdlib/src/registry.rs:1158`, `crates/nvs-stdlib/src/lib.rs:348`. The named
      check is `a_totp_window_is_bounded_and_a_replay_is_refused`.
- [ ] **`Core\Jwt`, whose algorithm comes from the key** — ADR 0060 § 4's first three bullets and § 5:
      `alg` is checked against the key and never consulted to choose, `exp` is mandatory with no flag,
      and verified claims come back **`tainted`** — the opposite of
      `crates/nvs-stdlib/src/signed_cookie.rs:115`'s `Qual::Launder`, and the contrast is worth a
      sentence in each module doc. The named check is `a_jwt_with_an_unexpected_algorithm_is_refused`.

## Backlog

- `examples/http.nvs` and the whole of stage 5 — `docs/agent/loop-goal.toml:2340`.
- Stage 6's two stores, `Core\Cache` local and shared — `docs/agent/loop-goal.toml:2369`.
- `cargo deny check` has never run on this machine; `cargo-deny` is not installed — `docs/adr/0051-standard-library-tiers.md` § 4.
- The `[context]` manifest edits listed under *State*, in `docs/agent/loop-goal.toml`.
