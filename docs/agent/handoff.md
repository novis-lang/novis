# Handoff

## State

**Goal 6, Stage 5: the service future is real, and a request runs as a peer task.**
`nvs_server::serve_connection`'s service is an `async move` that starts the isolate
(`Isolate::start`), answers `Pending` while `Running::finished` is false, and joins only once it is
true — so the connection's own task stays free to go round `hyper`'s dispatcher loop, which is where
a body's bytes will come from. `Running` gained that non-parking question and `abandon` beside
`join`; `Peer`'s drop (`crates/nvs-server/src/serve.rs:368`) is ADR 0072 § 4 on the path a join never
reaches, and `Started::abandon` is the one call that may not wait — `nvs_runtime::Teardown` is how it
tells.

**Nothing supplies a body yet, and that is now the only thing in the way.** The shape objection is
gone: a pull may park the *isolate*, and the connection's next poll delivers. What is missing is the
reader over `hyper`'s `Incoming`, which is item 1 below. `Inbound::has_body` is false on every request
this server serves until then, so `examples/upload.nvs` — the failing acceptance check — stays failing
through items 1 and 2.

**`[context]` gaps, unchanged:** `adrs` selects no section of ADR 0105 (items 1-3 need §§ 3, 5, 6 and
its *Verification*) and no § 1 of ADR 0138; spec § 15 has no `spec` selector.

## Next group

**The body's supply path, end to end.** One file set: `crates/nvs-server/src/serve.rs`,
`crates/nvs-cli/src/serve.rs`, `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-stdlib/src/request.rs`.

- [ ] **The service future supplies the body, and the door attaches it** — a `RequestBody` over
      `hyper`'s `Incoming` at `crates/nvs-cli/src/serve.rs:304`, whose comment at
      `crates/nvs-cli/src/serve.rs:312` states what is owed. **The `Incoming` may not travel to the
      isolate**: it is polled with the connection's `Context`, and the isolate is a different task —
      so the reader is a shared cell plus a wake pair, and the connection's side of it goes into the
      wait at `crates/nvs-server/src/serve.rs:555`, which stops being "is the peer finished" and
      becomes "is it finished, and does it want a chunk". The isolate's pull parks its own task
      (`nvs_runtime::RequestBody`'s doc at `crates/nvs-runtime/src/ctx.rs:4556` is the rule);
      `Ctx::set_body` at `crates/nvs-runtime/src/ctx.rs:4486` is where it lands. ADR 0105 § 5's two
      caps are the supplier's to enforce, since it is the only thing counting bytes.
- [ ] **`Core\Request::body()` and `bodyStream()`** — spec § 15 and ADR 0105 § 3's three ways to read
      one, over the seam item 1 lands. `crates/nvs-stdlib/src/request.rs:163` is the row block and its
      module doc at `crates/nvs-stdlib/src/request.rs:13` already names all three members.
- [ ] **`Core\Request::files()`, and `examples/upload.nvs` runs** — ADR 0105's lazily yielded parts,
      the acceptance check this goal has been failing on: `parts=2 / field=title / file=report.pdf /
      saved 4096 bytes / no temp file`. The example states the whole contract in its own header
      (`examples/upload.nvs:7`) and the rows go beside item 2's at
      `crates/nvs-stdlib/src/request.rs:163`. Its load-bearing case is bounded resident memory against a body far
      larger than any in-memory bound (`docs/plan/m7.md`'s *Verify*), so the part reader may not
      accumulate any more than the supplier may.

## Backlog

- ADR 0097 § 6's forwarded-header walk — one line to change, named at `crates/nvs-server/src/serve.rs`'s
  `scheme` (ADR 0097 § 6).
- ADR 0083's WebSocket upgrade over `ConnectionIo::into_stream` (`crates/nvs-server/src/io.rs`).
- Stage 9's state-bleed suite across the isolate boundary (`docs/plan/m7.md`).
- `docs/agent/loop-goal.toml`'s `[context]` gaps above, which only a session that hits them can add.
