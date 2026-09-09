# Handoff

## State

**Goal 41 — a response body written over time, and the two doors onto it — has just started; nothing
of it has landed yet.** The previous goal's whole acceptance list is this goal's floor.

**There is no streaming body anywhere in this workspace.** `Answer` at
`crates/nvs-server/src/serve.rs:116` is an `Option<Bytes>` with an exact `size_hint`: one frame,
always `Content-Length`, never chunked. Its own doc comment says so — "a body this server already
holds in full" and "the one case that never streams". Nothing can write a byte to a client after the
head has gone out, which is why `crates/nvs-server/src/serve.rs:580` records that the
`200 text/event-stream` is "still a later slice's" and an event stream's isolate runs with
`Output::Capture` into a buffer nothing reads.

**So the SSE door is a door onto nothing.** `Core\Sse::upgrade` is registered
(`crates/nvs-stdlib/src/sse.rs:63`, wired at `crates/nvs-stdlib/src/registry.rs:1591`); every request
carries the cell (`nvs_runtime::SseSlot`, `crates/nvs-runtime/src/ctx/inbound.rs:1571`, offered at
`crates/nvs-server/src/serve.rs:811`); the both-cells-filled `500` is decided; and the isolate is
started at `crates/nvs-server/src/serve.rs:938` after the request is joined and its arena released.
`crates/nvs-stdlib/src/sse.rs:47` § *What is not here yet* is the honest list: `current()`, `send`,
and the response they would write into.

**ADR 0083 § 5 calls a streaming response something "M7 already builds".** It does not and never did.
The record is frozen and is **not** edited; the record this goal opens is where that is corrected.

**One trap is worth more than the rest of this paragraph.** `Phase::Write`
(`crates/nvs-server/src/io.rs:116-129`) bounds a response write by `write_idle`, which defaults to 30
seconds at `crates/nvs-config/src/server.rs:97` and which an operator can set to anything. An event
stream sits in that phase for hours. A heartbeat written as a constant works until someone writes
`write_idle_timeout = "5s"`, after which every stream in the deployment dies at five seconds with
nothing in the log — so the heartbeat is **derived** from `write_idle`, never configured beside it.
The socket path never hit this because `hyper` stops framing after the `101`; an event stream leaves
`hyper` in charge, so the phase machinery stays live under it.

The design decisions are settled and are **not** to be re-litigated — the goal's § *Standing
decisions* is the list, and it carries the reasoning and the fallback for each. The four that a
session will otherwise want to re-open: **`receive()` is topics-only** and § 5's "no receive" means
"no peer"; **one `Core\Sse` type across both doors**, with `receive()` throwing in a streaming
response; **`send(mixed $data, …)`** with a `string` going out raw and anything else JSON-encoded,
the `send`/`sendJson` split having been considered and rejected; and **`$data` accepts `tainted`
while `$event` and `$id` refuse it**.

## Next group

**Stage 0 and stage 1 together.** Stage 0 is one paragraph in the record this goal opens and shares
no file with anything; stage 1 is the keystone and every stage after it sits on the cell. Take them
as one slice — the record is opened in stage 0 either way, and stage 1 has nothing to say until the
cell exists.

- [ ] **Open the record.** One new record, and **do not name a number** — it is claimed by the file
      that lands, one above the highest in `docs/decisions/`. Its stage 0 paragraph: ADR 0083 § 5's
      streaming response was never built, so § 5's line has been drawn between two unbuilt things,
      and this goal builds both sides of it. 0083 itself is frozen and is not touched.
- [ ] **Write the response-body cell in `nvs-runtime`**, mirroring `crates/nvs-server/src/body.rs`
      — read that module's doc first, in full: it is the same seam in the other direction and it
      already argues why a wake pair beats a queue and why an `Rc` is sound here. `Emit` on the
      isolate, `Drain` on the connection, **one chunk in flight**, the producer parking until the
      consumer has taken it.
- [ ] **Arm the send timeout on the `Emit` side** from `bounds::Connection::send`, passed in rather
      than named in the cell. A consumer that stopped reading must close its producer, not park it
      forever — that is the whole of the backpressure story and there is no second mechanism.
- [ ] **Make `Answer` two-valued** at `crates/nvs-server/src/serve.rs:116`. The whole-body variant
      keeps its exact `size_hint` and its `Content-Length` untouched; the streaming variant reports
      no exact size so `hyper` chunks it. `Answer::bytes` keeps answering the whole variant's buffer
      — `crates/nvs-server/src/statics.rs`'s cases are assertions about that method and must not
      move.
- [ ] **Rewrite `Answer`'s doc comment as a whole.** It currently argues that a dependency was not
      taken because this is "the one case that never streams". That sentence is about to be false,
      and a comment is rewritten rather than amended.
- [ ] **Expect no Novis surface in this group.** Stage 1 is provable from Rust alone and that is why
      it is first; a member added here is a member written before the thing it writes into exists.
- [ ] **Stage 2 is the next group and shares almost none of this file set** — it is one new module,
      `crates/nvs-server/src/sse.rs`, and a function over bytes. Read
      `crates/nvs-server/src/socket.rs`'s `Framed` for the shape it is a sibling of. Its first item
      is the normalization: the client parser terminates a line on `\r\n`, on `\r` **and** on `\n`,
      so a payload carrying a lone `\r` splits into two events on the far side unless every payload
      is normalized to `\n` before it is split.

## Backlog

Nothing yet.
