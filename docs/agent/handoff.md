# Handoff

## State

**Goal 6, Stage 3: § 10's mount is pinned to routing, and § 8's accept loop is bounded.**
`[[server.mount]]` holds ADR 0097 § 3's five routing keys and nothing else —
`crates/nvs-config/tests/tree.rs`'s `a_mount_carries_no_policy_of_its_own` asserts that on both
sides, refusing `mode`, `limits`, `limits.hard`, `capabilities` and `log` under a mount *while the
same directive parses under ADR 0104's `[[app]]`*, so a refusal that came from the directive being
unspellable anywhere fails there too. Three sites still routed policy to a mount and now read as
§ 10 decided: its own TOML example (`path`/`root`, keys § 3 does not give a mount), ADR 0097's
`Amends:` clause for 0091, and 0091's own mixed-application row.

**The accept loop backs off** — ADR 0106 § 8, and `crates/nvs-server/src/serve.rs`'s
`AcceptBackoff` is the whole of it. `EMFILE`/`ENFILE`, and Windows' `WSAEMFILE`, is waited out
rather than returned — 10ms doubling to 1s, reset by the first accept that succeeds — and reported
**once per 60s window**, which is § 8's other half. Every other `accept` error still ends the loop,
and that bound is asserted on both sides. The note is handed back as a `String` and
`serve_on_this_core` gained a `report` sink for it, on `Ceiling::clamp_note`'s precedent: this crate
is given a socket and not a logger, so `nvs serve` is what writes it to stderr.

**Unchanged limits.** There is no drain state anywhere in the tree — `health_path` has no reader in
`nvs_config::server`, and there is no `Core\Server` class in `nvs-stdlib` at all — so the next group
is all of it, and the goal's `is_draining_answers_during_a_graceful_shutdown` is open until then. No
wedged-core watchdog (ADR 0106 § 7); a served request's context still carries no configuration
snapshot; a Unix-domain `listen` entry still classifies and is then refused in the CLI. The driver's
`native examples/upload.nvs` failure is stage 5's frozen `want` ahead of the frontier — the playbook
bullet that owns it is under *Divergences and refusals already pinned* — and is not a regression.
`[context]` gaps this session paid for: `adrs` printed `0097 §2`, `§4` and `0106 §13` but the item
needed `0097 §3` and `§10`, and the second slice needed `0106 §8`; `modules` names no `nvs-cli`
pattern, though the accept loop's only caller is `crates/nvs-cli/src/serve.rs`.

## Next group

**§ 5's `health_path`, and the drain it reports.** One file set:
`crates/nvs-config/src/server.rs`, `crates/nvs-server/src/mount.rs`,
`crates/nvs-server/src/serve.rs`, `crates/nvs-cli/src/serve.rs`.

- [ ] **`health_path` is read into what a server starts on.** ADR 0097 § 5 — off by default, so no
      URL is silently reserved; a written one is an absolute path and nothing else.
      `crates/nvs-config/src/server.rs:118` is `waits_for`, the shape a `[server]` key is read
      through, and `crates/nvs-config/src/server.rs:99` is the boot refusal beside it.
- [ ] **The health path answers before the mount table.** ADR 0097 § 5 and § 4's five steps: it is
      checked ahead of step 1, so no mount can shadow it, and it performs no dependency checks and
      reports no version. `crates/nvs-server/src/mount.rs:171` is `Table::select`, and
      `crates/nvs-server/src/mount.rs:262` is the `What` a selection answers with.
- [ ] **Draining is a state the accept loop holds.** ADR 0097 § 5 — `200` while accepting, `503`
      while draining, empty body. `crates/nvs-server/src/serve.rs:396` is `serve_on_this_core` and
      its `keep_serving` seam, and `crates/nvs-cli/src/serve.rs:282` is the
      `ControlFlow::Continue` forever that the drain replaces. This is the slice that pins the
      goal's `is_draining_answers_during_a_graceful_shutdown`.
- [ ] **`Core\Server::isDraining()` gives an application the same fact.** ADR 0097 § 5's last
      sentence, as the five edits — there is no `crates/nvs-stdlib/src/server.rs` yet, so this is a
      new class as well as a member. `crates/nvs-stdlib/src/registry.rs:1279` is the roster it joins.

## Backlog

- The wedged-core watchdog — ADR 0106 § 7, named by ADR 0097 § 5.
- A served request's context carries no configuration snapshot — `crates/nvs-server/src/serve.rs`.
- ADR 0097 § 6's forwarded-header walk: `trusted_proxies` is parsed and read by nothing.
- A Unix-domain `listen` entry classifies and is then refused in the CLI — ADR 0097 § 5.
- `examples/upload.nvs` against stage 5's frozen `want` — ADR 0105, playbook bullet owns it.
