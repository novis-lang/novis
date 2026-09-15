# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
stages 3 and 4 are complete. **Stage 5 is one group from done**: every verb is a plan of actions, and
both appliers now apply one.

`registration::scm::Scm` (`crates/nvs-cli/src/service.rs:1841`) is the Windows half — `CreateServiceW`
plus the configuration levels for the description, delayed auto-start, `PRESHUTDOWN` and the failure
actions; `RegCreateKeyExW`/`RegDeleteTreeW` for the event-log source; `SetEntriesInAclW` over the DACL
`crates/nvs-config/src/trust.rs` reads, for a grant and its revoke. `registration::Systemd`
(`:1676`) is the Linux half — the unit write, its remove, and `systemctl` by argv with no shell.
Three steps answer for the state they ask for rather than for the call (a service already gone, a stop
of one that is not running, a source that was never registered), so an uninstall after an interrupted
install still leaves nothing behind.

**`Systemd` is deliberately not `#[cfg(unix)]`** — `verify.py` has no Linux leg, and a write, a remove
and a child process are `std` everywhere, so compiling it on both platforms is what keeps it compiled.
`Scm` has no such choice and is type-checked on Windows only.

**What is not on disk is the `ServiceCommand` variants beside `unit`**, so `nvs service --help` still
names one verb and stage 5's first check is still red. Nothing is blocked.

## Next group

**Stage 5: `nvs service`, the surface over the two appliers** — one file set:
`crates/nvs-cli/src/main.rs` and `crates/nvs-cli/src/service.rs`.

- [ ] **`ServiceCommand` gains `install`, `uninstall`, `start`, `stop` and `status` beside `unit`,
      each one a `Site` over the applier `Platform::host()` names** — the enum is
      `crates/nvs-cli/src/main.rs:961` and the dispatch that currently reaches `unit` alone is
      `crates/nvs-cli/src/main.rs:1175`; the front doors are `install`
      (`crates/nvs-cli/src/service.rs:1614`), `uninstall` (`crates/nvs-cli/src/service.rs:1634`) and
      `control` (`crates/nvs-cli/src/service.rs:1650`), and the appliers are
      `crates/nvs-cli/src/service.rs:1676` and `crates/nvs-cli/src/service.rs:1841`. `--dry-run`
      is `perform`'s own parameter (`crates/nvs-cli/src/service.rs:1581`).
      `rule:packaging/a-service-is-one-stored-argv`.
- [ ] **`nvs service run` is the foreground spelling of "start it as the manager would"** — both
      `image_path` and `unit()` put the argv in directly, so a manager never invokes `run`; what it
      needs is the stored argv read back, and `decode` (`crates/nvs-cli/src/service.rs:471`) is
      already the Windows half. The drain, the `PARAMCHANGE` reload and the event-log records are
      `rule:packaging/a-service-answers-its-manager`'s table, entered through
      `Drain::process().begin()`.
- [ ] **The stage's cases, against the recording manager** — the `cargo-named` check at
      `docs/agent/loop-goal.toml:10201` names them and `Recording` is
      `crates/nvs-cli/src/service.rs:2404`. Nothing in one touches the real SCM or `systemctl`;
      `service_status_reports_in_flight_and_drain_progress_from_the_control_socket` is the one that
      drives the control socket instead, and `crates/nvs-cli/src/ctl.rs` holds the client half.

## Backlog

- `crates/nvs-cli/src/service.rs` is past [doc-style.md](doc-style.md)'s ~1,500-line target; the
  spec-shaped seam is `registration` and its two appliers.
- `docs/agent/goals/23-per-core.md:87-89` still says `serve.rs` refuses `Listen::Unix` — retired
  goal's prose, owned by goal `plan-truth`.
- `rule:http-server/a-unix-socket-listener` is still `designed`; stage 13 flips it with `guardedBy`
  filled from stage 4's two cases.
- `cargo clippy --all-targets -- -D warnings` on the Linux leg fails on
  `crates/nvs-db/src/mysql.rs:325` and `pg.rs:375` (`large_enum_variant`); nothing gates it, because
  `verify.py` has no WSL leg.
- The unowned gaps at `crates/nvs-server/src/route.rs:30`, `bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
