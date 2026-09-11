# Handoff

## State

**Goal `event-streams` — stages 0 and 1 are landed, and nothing of stages 2 to 6 is.** A response
body can now be written over time in Rust; no Novis program can reach one yet.

**The cell is `crates/nvs-runtime/src/stream.rs`**: `stream::open(send_timeout) -> (Emit, Drain)`,
one chunk in flight, the writer parked until the connection has taken it, the wait bounded by the
send timeout and ended at once by a dropped consumer. Its module doc is the home of why it holds two
kinds of wake and one `Rc`. **Neither half is re-exported at the crate root** — `nvs_runtime::Drain`
is the server's drain bit — so both are written `stream::` at every call site.

**`Answer` is two-valued** at `crates/nvs-server/src/serve.rs:116`: `Whole(Option<Bytes>)`, whose
exact `size_hint` and `Content-Length` are untouched, and `Streaming(stream::Drain)`, which reports
no exact size and is chunk-framed on the wire. `Answer::stream(&bounds::Connection)` is the only
place the send timeout is read, and `Answer::bytes` still answers the whole arm's buffer, which
`crates/nvs-server/src/statics.rs`'s cases depend on.

**Nothing outside a test constructs a streaming `Answer`.** No route answers one and no `Core` member
opens one, so `Core\Sse::upgrade` still starts an isolate with nothing wired to a body
(`crates/nvs-server/src/serve.rs:938`). The doors are stages 3 and 4.

**Record `docs/decisions/0176.md` is open** with the stage 0 correction and nothing else: its
`changes:` names no rule yet, and stage 6 is where the rules it modifies are named on both sides.

`Phase::Write`'s trap (`crates/nvs-server/src/io.rs:116-129`) is still ahead of this work and
unchanged — a heartbeat is derived from `write_idle`, never configured beside it. The goal's
§ *Standing decisions* stayed authoritative; none of it was re-opened.

## Next group

**Stage 2: the framing, as a function over bytes** — one file set: `crates/nvs-server/src/sse.rs`,
new and registered beside `pub mod socket;` at `crates/nvs-server/src/lib.rs:116`, with
`crates/nvs-server/src/socket.rs:148`'s `Framed` as the sibling shape it is written against. It
shares no file with stage 1 and needs no server to prove.

- [ ] **Normalize every payload to `\n`, then split it, one `data:` line per line.** The client
      parser terminates a line on `\r\n`, on `\r` **and** on `\n`, so a lone `\r` in a payload splits
      into two events on the far side; normalizing first is what makes the taint call in the goal's
      § *Standing decisions* sound rather than hopeful. Shape it on
      `crates/nvs-server/src/socket.rs:148`.
- [ ] **The three refusals, each named in its own message** — an `$event` or `$id` carrying `\n`,
      `\r` or NUL, and an empty `$data`, all `LogicError` and all the framing's rather than the
      member's. The reasoning is the goal's § *Standing decisions*; `docs/decisions/0176.md` is where
      stage 6 writes it down. Register the module at `crates/nvs-server/src/lib.rs:116`.
- [ ] **The optional fields, the terminator and the keepalive** — `event:`, `id:`, `retry:` as ASCII
      milliseconds, one blank line to terminate, no BOM ever. The keepalive `:\n\n` is written by the
      connection side alone, since only it knows the wire is idle;
      `rule:concurrency/connection-bounds-are-finite` is the table its period joins in stage 5, next
      to `crates/nvs-server/src/bounds.rs:86`.

## Backlog

- Stage 3 is where a streaming `Answer` first reaches a route, and it opens `serve.rs` again —
  `docs/agent/loop-goal.md` § *Stage 3*.
- `0176`'s `changes:` block and the four rules it modifies are stage 6's — `docs/agent/loop-goal.md`
  § *Stage 6*.
- The derived heartbeat reads `write_idle` at `crates/nvs-config/src/server.rs:97` — stage 5.
- `Emit` reports a truncated stream as a clean end; a writer that failed has no separate word yet —
  `crates/nvs-runtime/src/stream.rs`.
