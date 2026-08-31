# Handoff

## State

**Goal 4 is running and stage 4's password half is closed.** `Core\Password::hash`, `::verify` and
`::needsRehash` are rows, cards and bodies in `crates/nvs-stdlib/src/password.rs`, whose module doc
is the one home for the parameters (Argon2id, v19, m=19 MiB, t=2, p=1 — OWASP's second
configuration), for what they spend (~19 MiB transiently per call, charged to the request through
`nvs_runtime::budget`, O(in-flight logins)), for why the salt is drawn through
`crate::random::draw` rather than `getrandom`, and for the two divergences from PHP: a stored value
that is not a hash **throws** where `password_verify` answers `false`, and `needsRehash` asks
whether a hash is **weaker** rather than whether it differs, so a stronger stored hash is never
downgraded. `verify` honours the stored `m` — that is what makes an old hash verifiable — under a
1 GiB ceiling, because otherwise one `UPDATE` is a denial of service.

**`Core\Password` is the second and last writer of `Qual::Reveal`**, ADR 0033 § 3's `secret`
admission. The roster is now four rows across two classes and it is closed by a test rather than by
a doc comment: `nvs-types`' `reveal_and_the_password_helpers_are_the_only_launderers_of_secret`
asserts the set, and separately that no other mark admits a `secret` at all. `registry.rs`'s `Qual`
doc and `quals.rs`'s two `Reveal` paragraphs are updated to say two classes rather than one.

**`argon2` 0.6 is a new dependency**, with `[workspace.dependencies]`'s comment carrying ADR 0051
§ 4's two answers and the honest cost: it adds eleven crates, three of which are a second copy of
the `digest` trait set at 0.11 beside the `digest 0.10` `Core\Hash` is pinned to.
`THIRD-PARTY-LICENSES.txt` is regenerated. **`cargo deny check` was not run — `cargo-deny` is not
installed on this machine**; every added crate is MIT/Apache-2.0, which `deny.toml`'s allow list
carries, and `multiple-versions` is `warn`, so the expected verdict is clean.

**The driver's acceptance failure has moved one member on.** `examples/crypto.nvs` now gets past
`Core\Password` and fails on `Core\Crypto::generateKey` — stage 4's second slice, which is the next
group's first item. The three fixtures still owing configuration are unchanged
(`examples/http.nvs` a stage-5 origin, `examples/logging.nvs` an `[[app]]` block, the rest their
`net.connect` grant), as is `nvs_runtime::commands`' gap 1.

**`orient.py`'s manifest is unchanged and still wrong in the same ways**: two dead
`[context] modules` selectors — `crates/nvs-host/src/pool.rs` and `crates/nvs-host/src/stream.rs` —
and it wants `crates/nvs-runtime/src/commands.rs`, `crates/nvs-types/src/defaults.rs`,
`crates/nvs-test/src/case.rs`, `crates/nvs-cli/src/main.rs`, `crates/nvs-runtime/src/ctx.rs` and
now `crates/nvs-stdlib/src/password.rs` added. It also did not print
`crates/nvs-stdlib/tests/conformance_coverage.rs`'s two gates, which cost this session three
rounds — the new playbook bullet is the cheap version. Nothing is blocked.

## Next group

**Stage 4 — the AEAD half, which is what `examples/crypto.nvs` now fails on.** The file set is
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/lib.rs` and one new module beside
`password.rs`, plus one `.nvst` per member trio. The fixture's six frozen lines are in
`docs/agent/loop-goal.toml`'s stage 4.

- [ ] **`Core\Crypto::generateKey`, `::seal` and `::open`, AEAD only** — a new
      `crates/nvs-stdlib/src/crypto.rs`, its class registered in
      `crates/nvs-stdlib/src/registry.rs:1108` and its symbols in
      `crates/nvs-stdlib/src/lib.rs:335`. No ECB, no unauthenticated CBC and **no
      cipher-name-as-string** (ADR 0051 § 3): the primitive is the library's, exactly as the
      password parameters are. `generateKey` answers a `secret bytes` and draws through
      `crates/nvs-stdlib/src/random.rs:473`'s seam like everything else; `open` throws on a
      forgery rather than answering, which is `examples/crypto.nvs:64`'s last line. Picking the
      crate is pre-authorized under ADR 0051 § 4 and owes the `[workspace.dependencies]` comment
      and `python tools/gen-attribution.py`.
- [ ] **Three conformance cases for the trio, and the two gates that demand them** — the floor is
      three cases per member and each `Fault` site owes an assertion or a declaration within 8
      lines, both checked by `crates/nvs-stdlib/tests/conformance_coverage.rs:331` and `:683`. The
      playbook's *Writing a test case* bullet has the shapes; write the members' refusal messages
      as fixed sentences from the start so a case can assert them.
- [ ] **`every_registered_cipher_is_an_aead`** — stage 4's first named check, listed at
      `docs/agent/loop-goal.toml:2293` and owed a home in `crates/nvs-stdlib`. Write it over the
      roster the new module registers rather than over one call, the way
      `crates/nvs-types/src/core_lib.rs:697`'s launderer test asserts a set.

## Backlog

- `Core\Jwt`, `Core\Csrf`, `Core\Totp`, `Core\SignedCookie` — ADR 0060's closed roster, and stage
  4's remaining three named checks.
- `a_verified_signature_does_not_launder_its_claims` in `nvs-types` — ADR 0060 § 5, and the second
  half of stage 4's `nvs-types` check.
- `cargo deny check` on this tree once `cargo-deny` is installed — `docs/agent/commands.md`.
- `nvs_runtime::commands`' gap 1: four `ArgConv` variants still `Unconverted`.
- `examples/http.nvs`, `examples/logging.nvs` and the `net.connect` grants — each owed by the stage
  that writes its configuration.
- `docs/agent/loop-goal.toml`'s `[context]` manifest — two dead selectors and six files it should
  name, listed in `## State`.
