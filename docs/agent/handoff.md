# Handoff

## State

**Goal 6, Stage 2: a request runs as goal 2's `Isolate`, and the connection it runs on is bounded
by a clock.** `nvs-server` answers an h1 request by running one isolate on the connection's own
coroutine, and ADR 0097 § 5's four waits — header, body-idle, write-idle, keep-alive — hold it to
a finite life at every moment of it. Five unit tests in `crates/nvs-server/src/serve.rs` drive it:
one answer, two over one kept-alive connection, a failed request answering `500`, a silent peer
closed by the header wait, and an idle kept-alive connection closed by the keep-alive wait.

**On disk.** The waits resolve in `crates/nvs-config/src/server.rs` (`Waits`, `waits_for`), which
`resolve.rs`'s boot pass calls, and `false` or `0` is refused there under the new `E0619`; the four
`[server]` keys are now `Setting` rather than `String` so `"10s"` and `10` read alike.
`crates/nvs-server/src/io.rs`'s `Phase` is which wait is in force and its § *The clock* is the home
of the two mechanisms — arm on phase change, refresh on bytes moved — and of the one phase change
the adapter cannot see, which `serve_connection` sets around the handler.

**One defect fixed a crate over, and it is load-bearing.** `nvs_host::NvsStream::answer` lifted the
task's timer entry on any `Ready` poll, so a write cancelled the read's deadline; `NvsStream::timed`
now records which interest filed it. The playbook bullet under *Writing Novis itself* is the whole
account. `crates/nvs-host/src/net.rs`'s module doc § *Every wait is bounded by a clock* states the
rule.

**Nothing user-reachable starts it, and the handler is still the caller's function.** There is no
`nvs serve`, no mount table, and no `max_in_flight` (ADR 0097 § 5's other two bounds are
process-wide and belong with the core count). The seam a real handler binds to is
`nvs_runtime::script::resolve` (`crates/nvs-runtime/src/script.rs:231`), which the CLI already
wraps at `crates/nvs-cli/src/script.rs:70`.

**The driver's acceptance sweep is truncated, and it is not a regression.** `native
examples/upload.nvs` is checked against stage 5's frozen `want`. The playbook's bullet on a
`loop-goal.toml` fixture check frozen ahead of the frontier owns it.

## Next group

**A server you can start, and a URL that selects a mount.** One file set:
`crates/nvs-cli/src/main.rs`, `crates/nvs-cli/src/script.rs`, `crates/nvs-server/src/serve.rs`,
`crates/nvs-config/src/server.rs`. `[context] adrs` should gain `0097 §3` before the mount slice;
§ 4 alone does not settle the boot-time glob expansion.

- [ ] **`nvs serve` starts one core and runs the loop.** The subcommand joins
      `crates/nvs-cli/src/main.rs:141`'s `Command`, reads `[server] listen` and hands
      `nvs_config::server::waits_for` (`crates/nvs-config/src/server.rs:90`) to
      `crates/nvs-server/src/serve.rs:286`'s `serve_on_this_core`, with the handler resolving
      through `crates/nvs-cli/src/script.rs:70`. ADR 0097 § 5's `listen` overloading and
      `--listen`/`--port` as the last word.
- [ ] **The handler selects a mount rather than constructing a path.** ADR 0097 § 4's five steps
      over the table § 3 expands at boot, in front of the handler at
      `crates/nvs-server/src/serve.rs:286`, so § 2's rule stays a set equality rather than a suite
      of attempted escapes.

## Backlog

- `max_in_flight` and the accept backoff — ADR 0097 § 5, and ADR 0106 § 13's arithmetic.
- Secure headers and the development rendering of a failure — ADR 0074 § 2, ADR 0092 § 3, both
  waiting on a mode reaching this loop.
- A request body that is actually read — ADR 0105's lazily yielded parts; today a request with a
  body ends its connection.
- Static file serving under a mount root — ADR 0097 § 4 steps 3 and 4.
- `nvs ctl reload` and the control socket — ADR 0078 § 6.
