# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal
opens). **Stage 3's client half is now whole.** `nvs ctl` has `reload`, `config` and `status`,
`--socket` addresses one of several servers on a host, and an answer that does not carry this
build's version is refused before its body is looked at.

`crates/nvs-cli/src/ctl.rs` is the whole of it and its module doc owns the drive: `hyper` frames
both halves, so the workspace manifest now takes its `client` feature and the lock file gained
`want` and `try-lock`. `nvs_config::control::connect` hands back a `Client` that answers
`WouldBlock` on both platforms rather than the blocking socket it used to alias — that is measured
and not defensive: with a blocking one the exchange sends nothing at all and hangs until the
server's idle bound fires.

**Nothing binds the endpoint yet.** `nvs serve` never calls `control::bind`, no thread runs
`nvs_server::control::serve`, and nothing implements `Controlled` over the running process, so the
three `ctl_*` cases stage 3's `-p nvs-cli` check names are unwritten — each needs a live server.
What is asserted today is the exchange itself, against the real `answer_connection` over a real
endpoint.

## Next group

**Stage 3: `serve` binds the endpoint and supplies `Controlled`** — one file set:
`crates/nvs-cli/src/serve.rs`, `crates/nvs-cli/src/ctl.rs`.

- [ ] **`serve` binds the control endpoint before any listener accepts** — in `run` at
      `crates/nvs-cli/src/serve.rs:124`, ahead of the `bind_all` at
      `crates/nvs-cli/src/serve.rs:1197`. `nvs_config::control::Address::of` over the booted tree,
      `bind(name, boundary)` so that a directory another account can write refuses the boot rather
      than being served from, and one thread of its own running `nvs_server::control::serve`
      (`rule:config/one-local-control-socket`).
- [ ] **The `Controlled` the running process supplies** — the trait and its five members are at
      `crates/nvs-server/src/control.rs:82`. `snapshot` and `reload` are the roots and unit cache
      `serve` already holds, with `nvs_config::control::reload` as the one function the socket,
      `systemctl reload` and `PARAMCHANGE` all end in; the count is the accept loop's and the drain
      bit is `nvs-runtime`'s (`rule:concurrency/a-drain-closes-a-connection-cleanly`).
- [ ] **The three `ctl_*` cases the stage's `-p nvs-cli` check names** — beside the exchange cases
      at `crates/nvs-cli/src/ctl.rs:390`, once a server with a bound endpoint can be booted in
      process. `ctl_refuses_an_answer_from_a_server_of_another_version` is already there and green.

## Backlog
- `nvs ctl`'s wait for a server that answers nothing is unbounded — the endpoint serializes
  operations, so bounding it is a `--timeout` decision rather than a constant
  (`crates/nvs-cli/src/ctl.rs`'s `exchange`).
- On Windows a control client that stops *reading* can still wedge the thread: a `WriteFile` into a
  full pipe buffer waits, where the Unix socket answers `WouldBlock` and the idle bound catches it.
  A `config` listing is the only answer big enough to reach it (`crates/nvs-config/src/control.rs`).
- `nvs_config::control::Endpoint::bound()` has no caller now that `accept` is on the endpoint.
- Stale prose — `docs/plan/m7.md`'s carrier list, `crates/nvs-cli/src/serve.rs:79-90`'s "no
  configuration" gap — belongs to goal `plan-truth`.
- The four `unowned` gaps at `crates/nvs-server/src/route.rs:30` and its siblings — goal
  `unowned-closures`.
- `Core\Metrics`'s three rows — goal `m8-stdlib-depth`.
