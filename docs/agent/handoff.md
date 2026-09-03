# Handoff

## State

**Goal 6, Stage 3 is closed.** ADR 0097 § 5's `[server]` block is whole — the four waits, the
ceiling as ADR 0106 § 13's arithmetic, `health_path`'s probe ahead of every mount, and now
`Core\Server::isDraining()`; § 10's mount holds no policy. Stage 4 is the frontier.

**The drain bit lives in `nvs_runtime::drain`**, because `nvs-stdlib` cannot see `nvs-server` and
both have to state the same fact. That module owns the atomic and its ordering.
`nvs_server::Draining` is the handle over it and owns the rule that the accept loop is the only
writer; `Core\Server::isDraining()` is the other reader, through `nvs_runtime::drain::is_draining()`.

**`Draining::new()` is gone, and the constructor is now a choice.** A server that *is* this process
takes `Draining::process()` (`crates/nvs-cli/src/serve.rs:228`); an accept loop inside a process
that is not only that server takes `Draining::detached()`. That distinction is load-bearing rather
than cosmetic: six `-p nvs-server` tests run a loop to completion, and a single process-wide bit
would have them reporting the test binary as draining while
`is_draining_answers_during_a_graceful_shutdown` asserts its first probe is `200`.

**Unchanged limits.** Nothing asks the loop to stop yet — `nvs serve` passes
`ControlFlow::Continue` forever — so a live probe reads `200` and `isDraining()` reads `false` for
the whole of a run; the `503`/`true` half arrives with ADR 0078 § 6's control socket.
`Core\Server` holds that one member: the request's own environment and `traceId()` (spec § 15) wait
on a served request's context carrying its own state, which is the same gap that keeps a
configuration snapshot off it. No wedged-core watchdog (ADR 0106 § 7); a Unix-domain `listen` entry
classifies and is then refused in the CLI. The driver's `native examples/upload.nvs` failure is
stage 5's frozen `want` ahead of the frontier and is not a regression.

**`[context]` gaps this session paid for**, both reported by the previous session and still open:
`adrs` prints `0097 §2`, `§4` and `0106 §13` but not **`§5`**, which is the section every Stage 3
item is specified by and had to be sliced by hand again; `modules` names no `nvs-cli` pattern
though `crates/nvs-cli/src/serve.rs` is the accept loop's only caller.

## Next group

**Stage 4 item 9: `Core\Response`'s five typed body members** — spec § 15 and ADR 0088 § 4. No
`Core\Response` is registered yet, so this starts with the class. One file set:
`crates/nvs-stdlib/src/response.rs` (new), `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/lib.rs`, `crates/nvs-server/src/serve.rs`.

- [ ] **`Core\Response` is registered, and `text` is its first member.** The five edits, against
      this session's class as the smallest worked example — `crates/nvs-stdlib/src/server.rs:35` is
      the row and the card, `crates/nvs-stdlib/src/registry.rs:1367` is the `CLASSES` line and
      `crates/nvs-stdlib/src/lib.rs:405` the `address` arm. Decide there what a written body *is*
      before the connection reads it: `crates/nvs-server/src/serve.rs:109`'s `Answer(Option<Bytes>)`
      is what a reply already carries and `crates/nvs-server/src/serve.rs:173`'s `Reply` is what a
      handler returns.
- [ ] **`json` and `bytes`, each setting its own `Content-Type`** — ADR 0088 § 4's reason the five
      are typed members and not one `write`: `json` serializes the value so a tainted one is safe,
      and `bytes`' content type is a sink. Same file set; the rows sit beside `text`'s in
      `crates/nvs-stdlib/src/response.rs`, registered at `crates/nvs-stdlib/src/registry.rs:1367`
      and reached through `crates/nvs-stdlib/src/lib.rs:405`.
- [ ] **`echo` and one of the five on one response is a compile error** — ADR 0088 § 4, the sixth
      HTML-only path. The sink already in force is `crates/nvs-runtime/src/ctx.rs`'s `OutputSink`,
      which `crates/nvs-server/src/serve.rs:173`'s `Reply` selects per request.

## Backlog

- Stage 4 item 10: the `echo` binding table enforced from the request — ADR 0088 § 3,
  `docs/agent/loop-goal.md`.
- Stage 4 item 11: ADR 0074 §§ 1-4's runtime half — secure headers, closed CORS, cookie defaults.
- Stage 5 item 15: uploads, which is the acceptance check the driver still reports failing —
  ADR 0105.
- `Core\Server`'s request environment and `traceId()` — spec § 15, waiting on a served request's
  context.
- `[context] adrs` needs `0097 §5` and `modules` an `nvs-cli` pattern —
  `docs/agent/loop-goal.toml`.
- ADR 0078 § 6's control socket is the only thing that will ever set the drain — until it lands,
  `503` and `true` are unreachable in a real run.
