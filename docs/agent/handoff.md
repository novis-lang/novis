# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens).
**Stage 3 is complete**: a `Type=notify` unit gets `READY=1`, `RELOADING=1`/`READY=1`, `STOPPING=1`
and `WATCHDOG=1` as `crate::service::Heartbeat` owns, and a reload now writes its outcome to
`Core\Log`.

`nvs_runtime::LogWriter` (`crates/nvs-runtime/src/ctx/output.rs`) is the `Ctx`-free door: `[log]
target`, `level` and `format` each keep **one** reader — the free `log_target`/`log_minimum`/
`log_format` that `Ctx::write_log_record` and `LogWriter::resolve` both call. A writer with no
request under it and a tree naming no target writes to stderr; `LogChannel::Output` is unreachable
there, because it is the program's output through the capture stack and there is no program. That
decision's home is `LogWriter`'s doc comment.

`Process::logged` (`crates/nvs-cli/src/control.rs:145`) writes one record after `READY=1`, resolved
from the tree now in force: `Info` with `applied`/`ignored`/`invalidated`, or `Error` with the
rendered refusal in a `refusal` field. The write's own failure is swallowed
(`rule:errors/engine-floor`). Nothing is blocked.

Stage 4 is next and is the acceptance check that is red. The goal prose says it shares no files with
stage 3, and `NvsUnixListener` already exists — what is missing is the binding and the mode.

## Next group

**Stage 4: the Unix listener, phase-gated** — one file set: `crates/nvs-cli/src/serve.rs`,
`crates/nvs-host/src/net.rs`, and one new case under `crates/nvs-cli/tests/`.

- [ ] **A Unix-domain `listen` entry is still refused where the rule admits it** —
      `crates/nvs-cli/src/serve.rs:1261` pushes every `Listen::Unix(path)` onto `unsupported` and
      refuses the whole set once. `rule:http-server/a-unix-socket-listener` admits it on Unix and
      refuses it on Windows, so the refusal narrows to `cfg(windows)` rather than disappearing, and
      it stays taken once over the whole set before any socket exists. The bind itself exists:
      `crates/nvs-host/src/net.rs:566` is `NvsUnixListener::bind`, and `:484` is the alias, both
      `#[cfg(unix)]`.
- [ ] **Nothing applies `[server] socket_mode`** — the directive is spelled at
      `crates/nvs-config/src/default.toml:394` and applies to Unix entries only. Decide where the
      chmod goes: `NvsUnixListener::bind` at `crates/nvs-host/src/net.rs:566` takes no mode today,
      and a mode applied after the bind leaves a window at the umask's mode.
      `rule:http-server/a-unix-socket-listener`'s last paragraph is why the value is load-bearing —
      `0660` trusts by group membership, and such a listener is implicitly trusted for the forwarded
      headers.
- [ ] **The acceptance check's one case, both platforms** —
      `a_unix_socket_listen_entry_binds_with_its_mode_on_unix_and_is_refused_once_elsewhere`, under
      `-p nvs-cli`, so a new file beside `crates/nvs-cli/tests/request.rs`. It binds and asserts the
      mode where the rule admits one and asserts the single refusal elsewhere, so it runs on every
      leg. The refusal half already has cases at `crates/nvs-cli/src/serve.rs:1673-1692` to read
      first.

## Backlog

- The acceptance check's stage-5 half (`nvs service` registration) is untouched —
  `docs/agent/loop-goal.md` § *Stage 5*.
- `crates/nvs-cli/src/serve.rs:57-62`'s module doc states the refusal as unconditional; stage 4
  rewrites it, not goal `plan-truth`.
- `nvs ctl config --origin` against the live snapshot —
  `rule:config/ctl-config-reports-the-live-snapshot`, stage 3's sibling, already landed in
  `nvs-server`; nothing here owes it.
