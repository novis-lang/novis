# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
stages 3 and 4 are complete. **Stage 5 is half landed**: every `nvs service` verb is a pure function
from a `Plan` to a list of actions, and one `Manager` applies them in order.

`service::registration` (`crates/nvs-cli/src/service.rs:1056`) is that half. `Action` is a union over
both platforms; `install_actions` (`:1362`), `uninstall_actions` and `control_actions` (`:1491`) are
the builders; `Stored` (`:1186`) is what the platform still holds, so an uninstall revokes exactly the
paths the install granted; `Manager` (`:1289`) is the one-method seam; `perform` (`:1579`) describes
the list instead of applying it for `--dry-run`, so the change-management artifact and the steps
cannot drift apart. The front doors are `install` (`:1612`), `uninstall` and `control`. Six cases
drive the recording manager, and the module doc owns why the list rather than the platform's answer to
it is what they assert.

**What is not on disk is the two appliers — `Scm` and `Systemd` — and the `ServiceCommand` variants
beside `unit`**, so `nvs service --help` still names one verb and stage 5's first check is still red.
Nothing is blocked.

## Next group

**Stage 5: `nvs service`, the appliers then the surface** — one file set:
`crates/nvs-cli/src/service.rs`, `crates/nvs-cli/src/main.rs`, and the root `Cargo.toml`.

- [ ] **`windows-sys` gains the feature groups the SCM half needs, and `Scm` applies the Windows
      actions** — `crates/nvs-cli/Cargo.toml:105` takes the workspace dependency, and the feature
      array to widen is the root manifest's `windows-sys` entry: add `Win32_System_Services`
      (`CreateServiceW`, `ChangeServiceConfig2W` for `PRESHUTDOWN` and the failure actions,
      `ControlService`, `QueryServiceStatusEx`), `Win32_System_EventLog` and `Win32_System_Registry`,
      because an event-log source is a subkey under
      `SYSTEM\CurrentControlSet\Services\EventLog\Application`. It implements `Manager`
      (`crates/nvs-cli/src/service.rs:1289`) for every `Action` the Windows builder emits, and
      `crates/nvs-config/src/trust.rs:212` is the DACL walk to model `Grant`/`Revoke` on.
      `rule:packaging/a-service-is-one-stored-argv`.
- [ ] **`Systemd` applies the Linux actions** — the same seam at
      `crates/nvs-cli/src/service.rs:1289`: `deliver` (`:1015`) already writes where `destination`
      says, and `Action::Systemctl` is run by argv with no shell
      (`rule:core-classes/process-is-argv-only`).
      `rule:packaging/the-unit-is-printed-and-install-is-the-opt-in`.
- [ ] **`nvs service` names every verb the rule lists, `unit` included** —
      `crates/nvs-cli/src/main.rs:961` is `ServiceCommand`, and
      `rule:packaging/a-service-is-one-stored-argv` is the closed list: `install`, `uninstall`,
      `start`, `stop`, `status`, `run` and `unit`, with `nvs install-service` as the hidden alias.
      Each arm builds a `Site` over `Platform::host()` and calls a front door at
      `crates/nvs-cli/src/service.rs:1612`; `--dry-run` is `install` and `uninstall`'s own flag.
      `status` adds stage 3's `GET /status` beside the manager's answer, and `run` is the one verb
      the seam has nothing for — the backlog's first item is what it needs.

## Backlog

- **`nvs service run` has no body, and the shape it needs is decided by what is already stored**:
  `image_path` and `unit()` both put the argv in **directly** (`ExecStart={exe} {argv…}`), so a
  service manager never invokes `service run` — a hosted `nvs serve` answers the SCM itself. `run` is
  therefore the foreground spelling of "start it as the manager would", and what it needs is the
  stored argv read back: `decode` (`crates/nvs-cli/src/service.rs:462`) is already the Windows half
  (`QueryServiceConfigW` → `lpBinaryPathName`), and the Linux half is the unit's `ExecStart`. Its
  drain, `PARAMCHANGE` and event-log records are `docs/agent/loop-goal.md` § *Stage 5*.
- `service_status_reports_in_flight_and_drain_progress_from_the_control_socket` drives the control
  socket rather than the recording manager; `crates/nvs-cli/src/ctl.rs` holds the client half.
- `docs/agent/goals/23-per-core.md:87-89` still says `serve.rs` refuses `Listen::Unix` because
  `NvsListener` accepts on TCP alone; it is a retired goal's prose, so goal `plan-truth` owns it.
- `rule:http-server/a-unix-socket-listener` is still `designed` — stage 13 flips it with
  `guardedBy` filled from stage 4's two cases.
- `cargo clippy --all-targets -- -D warnings` on the Linux leg fails on `crates/nvs-db/src/mysql.rs:325`
  and `pg.rs:375` (`large_enum_variant`, the `NvsTls<NvsTcp>` arm against the `cfg(unix)` one). It
  predates this stage and nothing gates it, because `verify.py` has no WSL leg.
- The unowned gaps at `crates/nvs-server/src/route.rs:30`, `bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
