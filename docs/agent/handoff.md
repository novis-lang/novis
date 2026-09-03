# Handoff

## State

**Goal 6, Stage 3: § 5's health probe answers end to end, and draining is the state it reports.**
`[server] health_path` is read by `nvs_config::server::health_path` — off unless written, an empty
string is that same off, and `E0623` refuses a value no request could carry (relative, a query or a
fragment, a literal space) or `/` alone, which would reserve every mount's own entry. It is refused
at boot beside the waits, in `nvs_config::server::validate`.

**The probe is step 0, ahead of § 4's five steps.** `nvs_server::mount`'s `Table` holds it and
`resolve` answers `Resolved::Health` before step 1 runs, so no mount shadows it — including a mount
at `/` and a static file sitting at the probe's own path, which is what the case asserts. `select`
and `resolve` now answer `Option<Resolved>`; `Resolved::selection()` is § 4 alone for a caller that
only asks about applications.

**`nvs_server::Draining` is the drain, and the accept loop is the only writer.** It is set the
moment `keep_serving` breaks — before the tail that parks on outstanding connections, so the drain
is announced while a proxy can still act on it — and `Reply::health` is the one place `200` and
`503` are chosen between. `is_draining_answers_during_a_graceful_shutdown` pins both answers from
one run.

**Unchanged limits.** Nothing asks the loop to stop yet: `nvs serve` passes `ControlFlow::Continue`
forever, so its probe reads `200` for the whole of a run and the `503` half arrives with ADR 0078
§ 6's control socket. There is no `Core\Server` class. **The seam that slice needs**: `nvs-stdlib`
does not depend on `nvs-server`, so `isDraining()` cannot read `nvs_server::Draining` where it
lives — the bit has to sit in a crate both see, and `nvs-runtime` is the one they share. No
wedged-core watchdog (ADR 0106 § 7); a served request's context still carries no configuration
snapshot; a Unix-domain `listen` entry still classifies and is then refused in the CLI. The
driver's `native examples/upload.nvs` failure is stage 5's frozen `want` ahead of the frontier and
is not a regression. `[context]` gaps this session paid for: `adrs` still prints `0097 §2`, `§4` and
`0106 §13`, and the item needed **`0097 §5`** — the previous session already reported `§3` and
`§10` missing, so the field is not being maintained; `modules` still names no `nvs-cli` pattern
though the accept loop's only caller is `crates/nvs-cli/src/serve.rs`.

## Next group

**§ 5's last sentence: an application reads the same drain.** One file set:
`crates/nvs-server/src/serve.rs`, `crates/nvs-runtime/src/lib.rs`,
`crates/nvs-stdlib/src/registry.rs`, a new `crates/nvs-stdlib/src/server.rs`.

- [ ] **The drain bit moves to the crate both sides depend on.** `nvs-stdlib` cannot see
      `nvs-server`, so the `AtomicBool` behind `Draining` belongs in `nvs-runtime` with
      `nvs_server::Draining` as the handle over it. `crates/nvs-server/src/serve.rs:264` is the type
      and `crates/nvs-server/src/serve.rs:489` is the accept loop's parameter; the one writer stays
      the loop, which is that type's own doc.
- [ ] **`Core\Server::isDraining(): bool` — the five edits.** ADR 0097 § 5's last sentence: an
      application answers a probe of its own with the same fact. There is no
      `crates/nvs-stdlib/src/server.rs` yet, so this is a new class as well as a member —
      `crates/nvs-stdlib/src/registry.rs:1279` is the roster it joins and
      `crates/nvs-stdlib/src/json.rs:1783` is the worked helper, its `address()` arm and its card.

## Backlog

- § 6's forwarded-header walk over `trusted_proxies`, fail-closed — ADR 0097 § 6.
- A wedged core is reported and shed — ADR 0106 § 7.
- The drain's trigger: `nvs ctl` is what asks the loop to stop — ADR 0078 § 6.
- A served request's context carries no configuration snapshot — ADR 0078 § 1.
- A Unix-domain `listen` entry classifies and is then refused in the CLI — ADR 0097 § 5.
- The probe is skipped by the access log, once there is one — ADR 0097 § 5.
