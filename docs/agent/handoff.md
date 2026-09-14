# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is on disk, and it is the only ADR
number this goal opens). **Stage 3 is open, and its server half is now whole**: the drain's bound,
the three operations, the endpoint's accept, and the one thread that drives them.

On disk now: `nvs_config::control::Endpoint::accept` blocks until a client is on the endpoint and
hands back a duplex stream that *borrows* it, so one client at a time is a fact about the types
rather than a discipline — a `UnixStream` on Unix, and on Windows the pipe instance itself, flushed
and disconnected on drop so the next accept takes the next client. **That stream does not block**: a
read with nothing behind it answers `WouldBlock` (a socket flag on Unix, a `PeekNamedPipe` on
Windows), because `hyper` polls for the next request before it writes the answer it is holding and a
read that waited there deadlocks against the client waiting for that answer.
`nvs_config::control::connect` is the client's half and stays blocking; on Windows it waits out a
busy instance in a bounded loop, which is what the Unix listen backlog does for free.

`nvs_server::io::Nonblocking` is `hyper`'s two IO traits over that stream — `WouldBlock` is
`Pending`, and a progress flag says whether a poll moved a byte — and
`nvs_server::control::answer_connection` is the drive: poll, wait a millisecond where the connection
is waiting on its peer, give up after 30 seconds of no byte in either direction. It is a loop of its
own and not `nvs_host::block_on`, because a park needs something to end it and this thread has no
reactor. `nvs_server::control::serve` is the accept loop over that, until the drain bit is set, with
keep-alive off so one connection is one operation.

**Nothing binds the endpoint yet, and there is no `nvs ctl`.** `nvs serve` never calls
`control::bind`, no thread runs `control::serve`, and nothing implements `Controlled` over the
running process — the roots, the unit cache, the in-flight count and the drain bit are all
`nvs-cli`'s. The earliest failing acceptance check is `nvs ctl --help`, which is the subcommand that
does not exist.

## Next group

**Stage 3: `nvs ctl`, then `serve` binding the endpoint** — one file set:
`crates/nvs-cli/src/main.rs`, `crates/nvs-cli/src/ctl.rs` (new), `crates/nvs-cli/src/serve.rs`.

- [ ] **`nvs ctl` and its three subcommands** — a `Ctl` variant beside the `Service` one at
      `crates/nvs-cli/src/main.rs:520`, its nested enum beside `crates/nvs-cli/src/main.rs:899`'s,
      and `mod ctl;` in the list at `crates/nvs-cli/src/main.rs:145`. `reload`, `config` and `status`
      over `nvs_config::control::connect`, `--socket` addressing one of several servers on a host,
      and `nvs_server::control::same_build` refusing an answer from another build
      (`rule:config/one-local-control-socket`). The request is `hyper`'s framing and not a string:
      check `crates/nvs-cli/Cargo.toml` for its `client` feature first.
- [ ] **`serve` binds it before any listener accepts** — at `crates/nvs-cli/src/serve.rs:519`, beside
      the `nvs_server::Draining::process()` the worker takes at `crates/nvs-cli/src/serve.rs:551`:
      `control::bind` under `control::boundary`, refused before anything listens, and a thread of its
      own running `nvs_server::control::serve` (`rule:config/one-local-control-socket`).
- [ ] **The `Controlled` that process supplies** — over the roots and the unit cache `serve` already
      holds at `crates/nvs-cli/src/serve.rs:519`, with the in-flight count and the drain bit; the
      reload is `nvs_config::control::reload` and nothing else
      (`rule:config/a-reload-names-what-it-could-not-apply`).

## Backlog
- On Windows a control client that stops *reading* can still wedge the thread: a `WriteFile` into a
  full pipe buffer waits, where the Unix socket answers `WouldBlock` and the idle bound catches it.
  A `config` listing is the only answer big enough to reach it (`crates/nvs-config/src/control.rs`).
- `nvs_config::control::Endpoint::bound()` has no caller now that `accept` is on the endpoint.
- Stale prose — `docs/plan/m7.md`'s carrier list, `crates/nvs-cli/src/serve.rs:79-90`'s "no
  configuration" gap — belongs to goal `plan-truth`.
- The four `unowned` gaps at `crates/nvs-server/src/route.rs:30` and its siblings — goal
  `unowned-closures`.
- `Core\Metrics`'s three rows — goal `m8-stdlib-depth`.
