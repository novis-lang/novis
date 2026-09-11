# Handoff

## State

**Goal `event-streams` — stages 0, 1 and 2 are landed, and nothing of stages 3 to 6 is.** A response
body can be written over time in Rust and an event can be framed into bytes; no Novis program can
reach either yet.

**The cell is `crates/nvs-runtime/src/stream.rs`**: `stream::open(send_timeout) -> (Emit, Drain)`,
one chunk in flight, the writer parked until the connection has taken it. `Emit::send` takes a
`Vec<u8>`, which is what stage 2's framing returns. Neither half is re-exported at the crate root,
so both are written `stream::` at every call site.

**The framing is `crates/nvs-server/src/sse.rs`**, new and pure: `sse::Event { data, event, id,
retry }` with `frame() -> Result<Vec<u8>, sse::Refused>`, and `sse::KEEPALIVE`. It holds no socket
and no clock, so it needs no server to prove, and its module doc is the home of why a payload is
normalized to `\n` before it is split. It is **not re-exported** at the crate root either —
`nvs_server::Event` would be a name with no subject — so it is written `sse::Event`.

**`Answer` is two-valued** at `crates/nvs-server/src/serve.rs:116`: `Whole(Option<Bytes>)`, whose
exact `size_hint` and `Content-Length` are untouched, and `Streaming(stream::Drain)`.
`Answer::stream(&bounds::Connection)` is the only place the send timeout is read.

**Nothing outside a test constructs a streaming `Answer` or frames an event.** No route answers one
and no `Core` member opens one, so `Core\Sse::upgrade` still starts an isolate with nothing wired to
a body (`crates/nvs-server/src/serve.rs:938`). The doors are stages 3 and 4.

**Record `docs/decisions/0176.md` is open** with the stage 0 correction and nothing else; stage 6 is
where the rules it modifies are named on both sides. `Phase::Write`'s trap
(`crates/nvs-server/src/io.rs:116-129`) is still ahead of this work and unchanged.

## Next group

**Stage 3: door two, the surface half** — one file set: the two registries and the type checker's
body-writer table. `serve.rs`'s head-early path (goal prose stage 3 item 12, the largest change in
the goal) is deliberately **not** in this group: it shares no file with these three and is the group
after.

- [ ] **`Core\Response::stream(string $contentType): Core\Response\Stream`, and that class's one
      member `write(string|bytes $chunk): void`.** The head goes out when `stream` is called and the
      body ends when the isolate does. `$contentType` is a sink and `$chunk` is not
      (`rule:security/sink-predicate`); the row goes beside `json`'s at
      `crates/nvs-stdlib/src/response.rs:239`, whose marks are the ones to copy.
- [ ] **`Core\Sse::stream(): Core\Sse`** — the same door with `text/event-stream` over it and
      `crates/nvs-server/src/sse.rs`'s framing behind it, answering the same handle `current()` will,
      so a helper taking `Core\Sse` works from either side (goal § *Standing decisions*, one type
      across both doors). The rows are `crates/nvs-stdlib/src/sse.rs:63`.
- [ ] **Both members are body writers, so `echo` beside one is a compile error.** One entry each in
      `crates/nvs-types/src/response.rs:68`'s `BODY_MEMBERS`, installed by
      `crates/nvs-types/src/check.rs:665` — `rule:security/response-body-is-one-typed-member`'s
      existing machinery and nothing new. This is what turns the stage's `nvs-types` check green:
      `echo_beside_a_response_stream_is_a_diagnostic`,
      `echo_beside_an_event_stream_is_a_diagnostic`, `two_body_writers_on_one_response_is_a_diagnostic`.

## Backlog

- `serve.rs` answers a head while the isolate still runs — goal prose stage 3 item 12, its own group.
- The three stage 3 `.nvst` cases need both halves landed — `docs/agent/loop-goal.toml:8441`.
- `setStatus` on a path that opens an event stream is refused — goal § *Standing decisions*.
- The goal's per-stage prose (`docs/agent/loop-goal.md` § *Stage N*, the numbered items) is not in
  the pack and no `[context]` field selects it; `sed -n '/## Stage 3/,/## Stage 4/p'` is one call.
- Carried gaps that outlive this goal: `docs/agent/carried-gaps.md`.
