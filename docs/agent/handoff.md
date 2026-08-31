# Handoff

## State

**Goal 4's stage 4 is closed, both halves.** `Core\Crypto::generateKey`, `::seal` and `::open` are
rows, cards and bodies in `crates/nvs-stdlib/src/crypto.rs`, whose module doc is the one home for
the construction (XChaCha20-Poly1305, and why the extended nonce rather than RFC 8439's 96-bit
one), the sealed layout (`nonce ‖ ciphertext ‖ tag`, a flat 40 octets of overhead), what it spends,
and why every way of failing to be authentic throws one message. `examples/crypto.nvs` prints its
six frozen lines, so the driver's acceptance failure is closed.

**A registry row can now declare `secret bytes`, which it could not before.** `CoreTy::SecretBytes`
and `CoreTy::SecretBlob(Qual)` are the unclassified and classified spellings, mirroring
`Bytes`/`Blob` one axis over; they are the first `CoreTy` variants carrying a *qualifier* rather
than an ADR 0088 classification, and the variant's own doc comment is the home of that difference.
`generateKey` returns one, so a key cannot be assigned to a plain `bytes`; `seal`/`open` declare
one, so a `secret` key crosses without any member removing the mark and **the `Qual::Reveal` roster
stays closed at two classes**. `Core\Hash::hmac`'s key took the same spelling — `hash.rs`'s module
doc had recorded that gap and named that parameter as wanting it first.

**Sealing a `secret` message is deliberately a written `Core\Secret::revealBytes` call.** The
`$message` parameter is an ordinary `Blob(Qual::Contagious)`, so a `secret` plaintext is refused
there like anywhere else. That is a decision, not an omission: `crypto.rs`'s module doc argues it,
and the alternative was making `Core\Crypto` a third launderer.

**`chacha20poly1305` 0.11 is a new dependency**, six crates — itself, `aead`, `cipher`, `inout`,
`poly1305`, `universal-hash` — with no second copy of anything, since `chacha20` was already in the
lock file as `rand` 0.10's own generator. `[workspace.dependencies]`'s comment carries ADR 0051
§ 4's two answers and that cost; `THIRD-PARTY-LICENSES.txt` is regenerated. **`cargo deny check`
was not run — `cargo-deny` is not installed on this machine**; every added crate is MIT/Apache-2.0.

**The manifest is unchanged and still wrong in the same ways** as the last handoff recorded: two
dead `[context] modules` selectors (`crates/nvs-host/src/pool.rs`, `crates/nvs-host/src/stream.rs`),
and it wants `crates/nvs-runtime/src/commands.rs`, `crates/nvs-types/src/defaults.rs`,
`crates/nvs-test/src/case.rs`, `crates/nvs-cli/src/main.rs`, `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-stdlib/src/password.rs` and now `crates/nvs-stdlib/src/crypto.rs` added. For the group
below, `[context] adrs` needs **ADR 0060 §§ 1, 4, 5** — none of it was printed this session and all
of it specifies the next three slices. Nothing is blocked.

## Next group

**Stage 4's protocol half — ADR 0060's closed four-entry roster, of which three are the named
checks still owing.** All three sit directly on the members this session landed and share one file
set: `crates/nvs-stdlib/src/registry.rs:1150` (the `CLASSES` tail, where `crate::crypto::CLASS`
now is), `crates/nvs-stdlib/src/lib.rs:323` (the `address` chain) and one new module each beside
`crates/nvs-stdlib/src/crypto.rs:140`. The class names are the spec's, at
`docs/spec/01-core-library.md:1101`: `Core\SignedCookie`, `Core\Csrf`, `Core\Totp`, `Core\Jwt`.

- [ ] **`Core\SignedCookie`, AEAD only and with key rotation** — ADR 0060 § 1's first bullet and
      § 4. Sign with the newest key, verify against several, and no unauthenticated mode in the
      API — which is `crates/nvs-stdlib/src/crypto.rs:153`'s `seal` and `:169`'s `open` with a key
      list over them rather than a second construction. Registered at
      `crates/nvs-stdlib/src/registry.rs:1150` and `crates/nvs-stdlib/src/lib.rs:323`. The named
      check is `a_signed_cookie_round_trips_and_a_tampered_one_is_refused`.
- [ ] **`Core\Csrf`, where the comparison is the only exposed operation** — ADR 0060 § 1's second
      bullet: a caller must not be able to write `==`, so there is no member answering the token
      for comparison. The binding is to a session *identifier* a caller passes, not to
      `Core\Session`, which is goal 6's. Draw through `crates/nvs-stdlib/src/random.rs:473`'s seam
      and compare through `crates/nvs-stdlib/src/hash.rs`'s constant-time `equals`. Same two
      registration anchors. The check is `a_csrf_token_is_bound_to_the_session_that_issued_it`.
- [ ] **`Core\Totp`, with a bounded window and no replay** — ADR 0060 § 1's third bullet. The
      base32 pair `docs/spec/01-core-library.md:630` names as existing "for TOTP" is already on
      disk at `crates/nvs-stdlib/src/encoding.rs:397`. Same two registration anchors. The check is
      `a_totp_window_is_bounded_and_a_replay_is_refused`.

## Backlog

- `Core\Jwt`, plus `a_verified_signature_does_not_launder_its_claims` in `nvs-types` — ADR 0060
  §§ 4-5, the algorithm from the key and never the token.
- Three fixtures still owe configuration: `examples/http.nvs` a stage-5 origin,
  `examples/logging.nvs` an `[[app]]` block, the rest their `net.connect` grant.
- `nvs_runtime::commands`' gap 1, recorded in that module's own doc.
- `cargo deny check` has never run on this machine — `docs/adr/0051` § 4 is what it enforces.
- `orient.py`'s `[context]` manifest, the six additions and two dead selectors above —
  `docs/agent/loop-goal.toml`.
