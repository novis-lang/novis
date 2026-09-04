# Handoff

## State

**Goal 6, Stage 5: the body's supply path is live at the Rust seam.** `nvs_server::body`
(`crates/nvs-server/src/body.rs`) splits an arrived body into two halves over one `Rc` cell with a
wake pair: `Supply` keeps `hyper`'s `Incoming` on the connection and is pumped from inside the
service future's poll, `Pull` is the `nvs_runtime::RequestBody` the isolate parks on. The door
attaches the pull to `Inbound` and hands `Supply` back in `Reply::Run`'s second field, because an
`Incoming` may only be polled with the connection's own context. Nothing is read before a program
asks, and one chunk is in flight at a time — the module doc is the whole argument.

**Two lib tests pin it** (`crates/nvs-server/src/serve.rs:1381`): a body written in two halves 50 ms
apart reaches the program whole, which can only happen across a park, and a bodiless request reaches
it carrying none. `upload_total` is `nvs_server::body::UPLOAD_TOTAL` — ADR 0105 § 5's default as a
constant until `[limits]` carries the row — refused before dispatch as `Reply::too_large()` where a
`Content-Length` declares it, and at the pull where a chunked body crosses it.

**No Novis program can read the body yet**, so `examples/upload.nvs` — the failing acceptance check —
stays failing through the group below. A body a program never reads is never drained either, so that
connection ends rather than keeping alive; `serve_connection`'s docs own why that is the fail-closed
direction.

**`[context]` gaps:** `adrs` selects no § 3 of ADR 0105 (every item below needs it) and no § 1 of
ADR 0138; spec § 15 has no `spec` selector, and it is `docs/spec/01-core-library.md:1053-1075`.

## Next group

**The three ways to read a body, in `Core\Request`.** One file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-runtime/src/ctx.rs`, `tests/conformance/core/`.

- [ ] **`Core\Request::body()`** — ADR 0105 § 3's first way and spec § 15
      (`docs/spec/01-core-library.md:1053`): pull to the end into one string, bounded by
      `request_body` (8M) rather than `upload_total`, and throw where it is crossed. The five edits
      are the row at `crates/nvs-stdlib/src/request.rs:218`, the card at
      `crates/nvs-stdlib/src/request.rs:337`, the `address` arm at
      `crates/nvs-stdlib/src/request.rs:358` and a helper beside
      `crates/nvs-stdlib/src/request.rs:699`; the reader is `Ctx::body` at
      `crates/nvs-runtime/src/ctx.rs:4498`, whose `&mut` borrow is § 8's exclusivity.
- [ ] **`Core\Request::bodyStream(): Iterable<bytes>`** — the same pull as chunks rather than one
      string, so nothing is bounded and nothing is resident past a chunk
      (`docs/spec/01-core-library.md:1068`). It shares the row/card/address roster above at
      `crates/nvs-stdlib/src/request.rs:218`, and the chunk contract it has to keep is
      `crates/nvs-runtime/src/ctx.rs:4540`.
- [ ] **`Core\Request::files()`, and `examples/upload.nvs` runs** — ADR 0105's lazily yielded parts
      over the same pull, at `crates/nvs-stdlib/src/request.rs:218`, with the multipart boundary
      found across chunks (`crates/nvs-runtime/src/ctx.rs:4571` is why it may not assume one arrives
      whole). This is the acceptance check.

## Backlog

- `[limits] request_body` / `upload_total` as real `nvs_config` rows, replacing
  `crates/nvs-server/src/body.rs`'s constant — ADR 0105 § 5.
- ADR 0097 § 6's forwarded-header walk; `crates/nvs-server/src/serve.rs`'s `scheme` is its one line.
- ADR 0102's route table and `Core\Request::route()` — docs/plan/m7.md.
- ADR 0083's WebSocket isolate over `tungstenite` — docs/plan/m7.md.
