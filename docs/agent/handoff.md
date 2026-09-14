# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is on disk, and it is the only ADR
number this goal opens). **Stage 3 is open**, and two of its three pieces have landed: the drain's
bound with the wake that makes a connection see it, and the control surface's three operations.

On disk now: `nvs_server::control::Operation` is `Reload`, `Config` and `Status`, each performed
with the method that is part of its name; `nvs_server::control::answer` is the whole of what a
connected client can cause, and every answer it builds — a refusal included — carries
`nvs-control-version`, which `same_build` is the one reader of. What the process supplies is the
`Controlled` trait: the reload, the live snapshot, the `Boot` keys the last reload left unapplied,
the in-flight count and the drain bit. `[server] drain_timeout` resolves into `nvs_config::Waits`
and `nvs_runtime::Drain` carries a wake registry (`Drain::wake_at_drain`, fired by `Drain::begin`
outside the lock).

**One renderer, two trees.** `nvs_config::audit::Audit` is the listing both `nvs config dump
--origin` and `ctl config` go through, because `rule:config/ctl-config-reports-the-live-snapshot`
has an operator diff the two — `of_files` takes a `Resolved`, `of_snapshot` takes a `Snapshot` and
the unapplied `Boot` keys. `nvs-cli`'s `dump` is now a one-line caller and its `leaves`/`flatten`
are gone.

**Nothing accepts on the endpoint yet, and nothing registers a drain wake.** `answer` has no
caller: `nvs_config::control::Endpoint` is created and dropped, and its `Bound` is a `UnixListener`
on Unix and a bare `Bound(HANDLE)` with no `ConnectNamedPipe` on Windows. That is the next item,
and both remaining stage-3 checks rest on it.

## Next group

**Stage 3: the endpoint's accept loop, then `serve` binding it** — one file set:
`crates/nvs-config/src/control.rs`, `crates/nvs-server/src/control.rs`, `crates/nvs-server/src/io.rs`,
`crates/nvs-cli/src/serve.rs`.

- [ ] **A connected control client, on both platforms** — `accept` on
      `crates/nvs-config/src/control.rs:165`'s `Bound`, handing back a duplex stream: `UnixStream`
      from the `UnixListener`, and on Windows a `ConnectNamedPipe` beside the
      `CreateNamedPipeW` at `crates/nvs-config/src/control.rs:429` with `Read`/`Write` over the
      handle. One instance at a time is the shape the rule already wants
      (`rule:config/one-local-control-socket`).
- [ ] **One endpoint, one thread, one operation at a time** — the blocking accept loop over that
      stream in `crates/nvs-server/src/control.rs:26`'s module, driving `hyper`'s http1
      `serve_connection` into `answer`, so `two_reloads_on_one_endpoint_are_answered_one_after_the_other`
      holds. A thread and not a runtime (`rule:concurrency/one-scheduler`); the IO adapter beside
      `crates/nvs-server/src/io.rs:1`'s, but over blocking std IO, so the future never parks.
- [ ] **`serve` binds it before any listener accepts, and refuses a writable directory** — at
      `crates/nvs-cli/src/serve.rs:519`, beside the `nvs_server::Draining::process()` the worker
      already takes at `crates/nvs-cli/src/serve.rs:551`; the directory check is the one every
      configuration file gets (`rule:config/one-local-control-socket`). `nvs ctl` and its three
      subcommands are the same item's other half, and `same_build` is what refuses a mismatch.

## Backlog

- The `nvs ctl` client half — `reload`, `config`, `status`, `--socket` — `docs/plan/m7.md`.
- `sd_notify` and the terminating-signal drain, stage 3's last piece — `docs/plan/m7.md`.
- Nothing registers a `Drain::wake_at_drain` yet; the accept loop takes one per connection —
  `crates/nvs-runtime/src/drain.rs`.
- `rule:config/one-local-control-socket` still reads *designed, not yet shipped*; it flips when the
  endpoint accepts — `docs/rules/config/`.
