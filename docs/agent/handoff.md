# Handoff

## State

**`Core\Mail` authenticates.** A `[mail.<name>]` block that sets `user` and `password` is sent
through `STARTTLS` and `AUTH PLAIN`; one that sets neither stays in the clear exactly as before.
The three decisions behind that are in `crates/nvs-stdlib/src/mail.rs`'s module doc § *A credential
asks for TLS*, which is their only home:

- **TLS is asked for by configuring the credential that needs it**, and where it is asked for it is
  *required and verified*. Opportunistic TLS was rejected in both its spellings: verified, it
  breaks the sidecar relay presenting an internal certificate an operator has no `nvs.toml` key to
  name yet; unverified, `nvs_host::tls` has no spelling for it and will not grow one. Encryption
  *without* authentication therefore has no key today — it belongs beside that unlanded anchor
  bundle, and the two are one configuration slice.
- **Half a credential is refused at `endpoint_of`**, before a socket opens, as a `RuntimeError`.
- **The upgrade refuses a non-empty read buffer**, which is the `STARTTLS` command-injection
  defence; see the playbook bullet.

`Session` now holds a `Wire` — `Plain(NvsTcp)` or `Secured(NvsTls)` — and `expect`/`command` answer
with the reply's lines, because `EHLO`'s lines *are* the extension list. `advertised` reads them,
skipping the greeting line, which is the one line that is not a keyword.

**No `.nvst` case covers any of this and none can**: every new path is past the `mail.send` grant
and needs a live endpoint, so the three Rust cases in `mail.rs` carry it — the same shape
`nvs_host::tls` already uses. `Core\Mail::send`'s three conformance cases are unchanged and still
meet the coverage floor.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs §§ 15-19. Those are `Core\Request` and its neighbours and
are goal 6's, so this check cannot pass inside this goal and is not a regression.

**`cargo deny check`'s `advisories` leg is red on a yanked `chacha20` that predates this goal** and
arrives through `rand`; `licenses`, `bans` and `sources` are green. See the playbook bullet.

**Two pack gaps, both still open.** `[context] modules` does not select
`nvs-runtime/src/terminal.rs`. And `[context] playbook` filters to the *item's* anchor paths, so a
group that also writes `.nvst` cases never sees the case-authoring bullets.

## Next group

**`Core\Storage`'s owed depth, over `crates/nvs-stdlib/src/storage.rs` and its conformance cases —
the file set is that module plus `tests/conformance/core/storage-*.nvst`. The module's own doc
§ *known gaps* and ADR 0082 § 2 specify all three.**

- [ ] **A key is one segment, asserted on both sides.** The bound that admits the last accepted key
      and refuses the first rejected one — `a/b`, a leading dot, an empty key — named together in
      one case, which is `conventions.md`'s *bound asserted on both sides* shape.
      `crates/nvs-stdlib/src/storage.rs:1` is the module doc that states the rule.
- [ ] **The four rows agree about a missing object.** One question asked of `get`, `delete` and
      `list` over a disk that has none, asserting they **agree** rather than what each answered.
      `crates/nvs-stdlib/src/storage.rs:1`.
- [ ] **`[storage.<name>]` with no `root`, and a `root` that is not a directory.** Two refusals with
      no case between them; `crates/nvs-config/src/tree.rs:484` is the block they read.

## Backlog

- Encryption without authentication for `Core\Mail` needs an `nvs.toml` key, beside
  `nvs_host::tls`'s unlanded anchor bundle — `crates/nvs-host/src/tls.rs` module doc.
- `AUTH LOGIN` and `XOAUTH2` have no spelling; `PLAIN` over TLS is the whole roster —
  `crates/nvs-stdlib/src/mail.rs` module doc.
- `[context] modules` does not select `nvs-runtime/src/terminal.rs` — `docs/agent/loop-goal.toml`.
- `[context] playbook` filters to the item's anchor paths, so case-authoring bullets never print —
  `docs/agent/loop-goal.toml`.
- `cargo deny check advisories` is red on a yanked `chacha20` via `rand` — `deny.toml`.
- Stage 10's `every_part_two_spec_member_is_registered` needs spec §§ 15-19, which are goal 6's —
  `docs/agent/loop-goal.toml`.
