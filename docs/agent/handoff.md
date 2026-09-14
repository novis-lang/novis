# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
stage 3 is complete. **Stage 4 is complete**: a `[server] listen` entry beginning with a separator is
bound as a Unix-domain socket where `rule:http-server/a-unix-socket-listener` admits one, with
`[server] socket_mode` on it, and on Windows the whole set is still refused once in
`addresses` before any socket exists.

The family stops being a question at the accept. `nvs_host::NvsConnection`
(`crates/nvs-host/src/net.rs:909`) is an enum over `NvsTcp` and `NvsUnix` carrying `set_deadline`,
`poll_read`/`poll_write` and `Read`/`Write`; `nvs_server::Listening`
(`crates/nvs-server/src/serve.rs:1716`) is the one method that differs — it answers the connection
and the `Arrival` — and `serve_on_this_core` is generic over it and `?Sized`, so `nvs serve` hands
each core a `Box<dyn Listening>`. `ConnectionIo` and `socket::Prefixed` hold an `NvsConnection`; the
enum-rather-than-generic decision's home is `NvsConnection`'s own doc.

`nvs_config::server::socket_mode_for` resolves the mode (`0660` unwritten) and refuses every
spelling that is not a permission set under `E0648`. `bind_one`
(`crates/nvs-cli/src/serve.rs:1489`) narrows the umask around the bind rather than only
`set_permissions`-ing afterwards, so the socket never exists wider than the mode asked for; its
comment owns why. Nothing is blocked.

## Next group

**Stage 5: `nvs service`, phase-gated** — one file set: `crates/nvs-cli/src/service.rs`,
`crates/nvs-cli/src/main.rs`, and the root `Cargo.toml`.

- [ ] **`nvs service` names every verb the rule lists, `unit` included** —
      `crates/nvs-cli/src/main.rs:961` is `ServiceCommand`, and
      `rule:packaging/a-service-is-one-stored-argv` is the closed list: `install`, `uninstall`,
      `start`, `stop`, `status`, `run` and `unit`, with `nvs install-service` as the hidden alias.
      The check is a `--help` whose `want` names all seven.
- [ ] **`windows-sys` gains the two feature groups the SCM half needs** —
      `crates/nvs-cli/Cargo.toml:105` is where this crate takes the workspace dependency, and the
      feature array to add `Win32_System_Services` and `Win32_System_EventLog` to is the root
      `Cargo.toml`'s `windows-sys` entry. Taken first: the install verb cannot compile without them.
- [ ] **Every verb is a `Plan` of `Manager` actions and every refusal runs before the manager is
      touched** — `crates/nvs-cli/src/service.rs:936` is the recording `Manager` the tests drive,
      `:402` is `image_path` and `:525` is `unit`. `rule:packaging/the-installer-is-a-sink` is why
      `plan` runs before any `Manager` call; the goal's § *Standing decisions* names the seam as the
      design and forbids a test touching the real SCM or `systemctl`.

## Backlog

- `nvs service run`'s `PARAMCHANGE`/`STOP` mapping and the event-log records — stage 5's second half,
  `docs/agent/loop-goal.md` § *Stage 5*.
- `docs/agent/goals/23-per-core.md:87-89` still says `serve.rs` refuses `Listen::Unix` because
  `NvsListener` accepts on TCP alone; it is a retired goal's prose, so goal `plan-truth` owns it.
- `rule:http-server/a-unix-socket-listener` is still `designed` — stage 13 flips it with
  `guardedBy` filled from this stage's two cases.
- `cargo clippy --all-targets -- -D warnings` on the Linux leg fails on `crates/nvs-db/src/mysql.rs:325`
  and `pg.rs:375` (`large_enum_variant`, the `NvsTls<NvsTcp>` arm against the `cfg(unix)` one). It
  predates this stage and nothing gates it, because `verify.py` has no WSL leg.
- The unowned gaps at `crates/nvs-server/src/route.rs:30`, `bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
