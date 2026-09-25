---
milestone: M7
---
# Loop goal 42 — A response body written over time, and the two doors onto it

`rule:concurrency/a-stream-that-outlives-its-request-is-a-connection` draws a line between two
spellings — a stream that ends with its request is a *streaming response*, one that outlives it is a
*connection isolate* — and **neither side of that line is built.** The server has one body type,
`Answer` at `crates/nvs-server/src/serve.rs:116`, which is an `Option<Bytes>` with an exact
`size_hint`: one frame, always `Content-Length`, never chunked. Nothing in this workspace can write a
byte to a client after the head has gone out.

So `Core\Sse::upgrade` is a door onto nothing. It is registered
(`crates/nvs-stdlib/src/sse.rs:63`, wired at `crates/nvs-stdlib/src/registry.rs:1591`), every request
carries its cell (`nvs_runtime::SseSlot`, `crates/nvs-runtime/src/ctx/inbound.rs:1571`, offered at
`crates/nvs-server/src/serve.rs:811`), the both-cells-filled `500` is decided, and the isolate is
started at `crates/nvs-server/src/serve.rs:938` — after the request is joined and its arena released,
which is `rule:concurrency/a-connection-is-a-root-isolate`'s ordering. A program that calls it today
opens a root isolate with nothing wired to a wire.

**This goal builds the body, and then spends it twice.** One cell in `nvs-runtime`, one framing
module in `nvs-server`, and the two doors the rule already names: `Core\Sse::stream` for a stream
that ends with its request, `Core\Sse::current` for one that outlives it, and
`Core\Response::stream` for the untyped case — a large export, a chunked file — that is the same
machinery with no event framing over it.

**It is M7's**, the milestone goal `server` carried, and it is the last piece of ADR 0083 that never landed.

## Why here

M7's last unlanded piece, and the one goal `server` left behind:
`rule:concurrency/a-stream-that-outlives-its-request-is-a-connection` draws a line between a
streaming response and a connection isolate, and **neither side of it is built**. `Answer` is an
`Option<Bytes>` with an exact `size_hint`, so nothing in this workspace can write a byte to a client
after the head has gone out — which makes `Core\Sse::upgrade`, registered and cell-carried and
started in the right order since goal `server`, a door onto nothing. It is placed after goal `resource-ceilings` because a
long-lived connection isolate is exactly the runaway shape that goal's ceilings exist to stop — an
event stream whose budget is declared and unenforced is priority 1 spent to buy priority 3 — and
after goal `config-is-written` because that is where the hand-written run ends. Nothing after it depends on it.

## What is wrong today, in one line each

1. **There is no streaming body.** `Answer` holds `Option<Bytes>` and reports an exact size, so
   `hyper` always sends `Content-Length` and never chunks. `crates/nvs-server/src/serve.rs:580` says
   so from the SSE side: until the door writes a `200 text/event-stream`, an event stream's isolate
   runs with `Output::Capture` and its bytes reach its own buffer.
2. **An SSE isolate cannot wait.** ADR 0083 § 5 gives it "`send` and no `receive`" and § 4 states
   there is deliberately no `Core\Topic::receive()`, so nothing in the surface can block on a
   published value — which is the one thing an event stream exists to do.
3. **`echo` in a connection isolate has no sink.** `rule:tooling/echo-always-has-a-sink` carries five
   rows and asserts "a context with no attached sink does not exist". A connection isolate is a sixth
   context and has none: `Output::Capture` into a buffer nothing reads.
4. **ADR 0083 § 5 says a streaming response is something "M7 already builds".** It is not, and it
   never was. The record is frozen rationale and is **not** edited; the new record this goal opens is
   where that is corrected.
5. **`Phase::Write` would kill every stream it allowed.** `crates/nvs-server/src/io.rs:123` bounds a
   response write by `write_idle`, which defaults to 30s at `crates/nvs-config/src/server.rs:97` and
   is operator-configurable. An event stream sits in that phase for hours.

## Stage 0 — the floor, and the one correction owed

The previous goal's whole acceptance list, unchanged, plus the single doc fact that is false before
anything here is written.

1. **Say what ADR 0083 § 5 got wrong, in the record this goal opens** — that the streaming response
   it treats as already built was never built, so § 5's line between the two spellings has been a
   line between one unbuilt thing and another. The record is the home; **0083 is frozen and is not
   touched**, and `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection` states a
   definition rather than a claim about the tree, so it is not wrong today either — it gains its
   "both spellings exist" sentence in stage 6, when they do.

## Stage 1 — the cell, and the body that reads it

The keystone. No Novis surface, no server route, provable in Rust alone — and everything after it
sits on it.

2. **`nvs-runtime`: the response-body cell**, `crates/nvs-server/src/body.rs:16`'s shape reversed.
   That module is the request body crossing between two tasks over one cell because `hyper`'s
   `Incoming` is polled with the *connection's* `Context` and the isolate is a peer task; the
   response direction has the same seam and the same answer. `Emit` on the isolate, `Drain` on the
   connection, a wake pair between them and **one chunk in flight** — the producer parks until the
   consumer has taken it, so backpressure is structural and nothing accumulates. An `Rc`, for
   `body.rs`'s own reason: both halves live on one core by construction.
3. **`Answer` becomes two-valued** at `crates/nvs-server/src/serve.rs:116` — the whole body it holds
   today, exact `size_hint` and `Content-Length` unchanged, or a `Drain` whose size is unknown and
   which `hyper` therefore chunks. `Answer::bytes` keeps answering the whole variant's bytes and
   answers empty for a stream; `crates/nvs-server/src/statics.rs`'s cases are assertions about that
   method and must not move.
4. **The send timeout is armed on the `Emit` side**, from `bounds::Connection::send`, so a consumer
   that stopped reading closes a producer rather than parking it forever. This is the one place the
   cell knows a duration, and it is passed in rather than named here.

## Stage 2 — the framing, as a function over bytes

Shares no file with stage 1's cell and needs no server to prove. `crates/nvs-runtime/src/sse.rs`, new,
the mirror of `crates/nvs-server/src/socket.rs`'s `Framed`, and **the only place in the workspace that
writes a `data:` line.** It sits one crate below its mirror because both halves reach it: the payload is
a `Core\Sse` member's, and `nvs-stdlib` cannot name `nvs-server` without closing a cycle.

5. **Normalize before splitting.** The client parser terminates a line on `\r\n`, on `\r` **and** on
   `\n`, so a payload carrying a lone `\r` splits into two events on the far side. Every payload is
   normalized to `\n` and then split, one `data:` line per line. A program cannot escape its own
   event, and that is what makes the taint decision in § *Standing decisions* sound rather than
   hopeful.
6. **`event:`, `id:`, `retry:`, and the blank line.** `retry:` is ASCII digits, milliseconds, as the
   wire spells it. The terminator is one blank line. No BOM is ever emitted.
7. **The refusals are the framing's, not the member's** — an `$event` or `$id` carrying `\n`, `\r` or
   NUL, and an empty `$data`. Each is a `LogicError`, each is named in the message, and the reasoning
   for all three is in § *Standing decisions*.
8. **The keepalive comment**, `:\n\n`, written by the connection side and never by the isolate. The
   connection is the only half that knows the wire is idle — the isolate may be parked in `receive()`
   — so this is a property of where it is written, not a preference.

## Stage 3 — door two: the stream that ends with its request

Before the connection door on purpose. A request-scoped stream ends with its request, so **a `.nvst`
case can assert its complete body bytes**, which makes stages 1 and 2 provable in-process instead of
only from a raw-socket Rust test.

9. **`Core\Response::stream(string $contentType): Core\Response\Stream`** and the class's one member,
   `write(string|bytes $chunk): void`. The head goes out when `stream` is called; the body ends when
   the isolate does.
10. **`Core\Sse::stream(): Core\Sse`** — the same door with `text/event-stream` over it and the stage
    2 framing behind it. It answers the same `Core\Sse` handle `current()` will, which is what lets a
    helper taking one work from either side.
11. **Both are body writers.** One row each in `crates/nvs-types/src/response.rs`, which
    `crates/nvs-types/src/check.rs:665` installs — so `echo` plus a stream is a **compile** error
    through `rule:security/response-body-is-one-typed-member`'s existing machinery and needs nothing
    new.
12. **`serve.rs` answers a head while the isolate is still running.** Today `Reply::Run` is awaited to
    completion and `answer(Completion)` builds the response from what it echoed. A streaming response
    returns the head as soon as the door is called and joins the isolate when the body ends. This is
    the largest single change in the goal and the one that touches paths no other item does.
13. **The request's budget still bounds it** — `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`
    is unchanged by a body that arrives in pieces, and a streaming response is not a connection and
    gets none of `bounds::Connection`'s numbers.

## Stage 4 — door one: the stream that outlives its request

Everything here is over stages 1–3 and only the lifetime differs.

14. **`Core\Sse::current(): Core\Sse`**, `crates/nvs-stdlib/src/socket.rs`'s `current` row one class
    over, refusing with a `LogicError` in every program that is not an event stream.
15. **`Core\Sse->send(mixed $data, ?string $event = null, ?string $id = null): void`** — the one way
    onto the wire, in both doors.
16. **`Core\Sse->receive(): ?Core\Sse\Message`** and the `Core\Sse\Message` class, `topic` and
    `value`. Topics only: the subscriber queue, `Reactor::remote_wake` and
    `rule:concurrency/a-connection-is-a-loop`'s overflow-closes-the-subscriber answer are reused from
    the WebSocket path unchanged. `null` for a client that went away, for an overflowed queue and for
    a drain, exactly as the socket's `receive` answers it.
17. **`Core\Sse->retry(Core\Time\Duration $after): void`** — one `retry:` line, and the way a program
    says "do not come back for an hour" ahead of a planned drain.
18. **Wire the door at `crates/nvs-server/src/serve.rs:936`**, where the cell is already taken and the
    isolate already started. What changes is `answered`: the `200 text/event-stream` replaces the
    request's own response, and the connection isolate's `Emit` is the body.

## Stage 5 — the bounds, and the trap under them

Needs both doors to exist before any of it can be asserted. `crates/nvs-server/src/bounds.rs`, whose
`Connection::default` is `rule:concurrency/connection-bounds-are-finite`'s whole answer and whose
destructuring test is what makes a new field fail to compile without one.

19. **`heartbeat` is derived, never configured**: `write_idle / 2`, floored at one second. A constant
    works until an operator writes `write_idle_timeout = "5s"` and every stream dies at five seconds
    with nothing in the log. One number, one home, correct under any tuning — and a test asserts the
    relation holds across every value `crates/nvs-config/src/server.rs`'s parser accepts.
20. **`idle` is not armed for an event stream.** It means "the peer said nothing", and an SSE client
    says nothing *ever*; arming it closes every stream on schedule. `lifetime`, `send`, `drain` and
    `max_open` are armed as they are for a socket. This is the one field the two doors read
    differently and `bounds.rs`'s module doc is where that is said.
21. **`message` is reused as the maximum event size.** One number for the largest thing a program
    hands the wire, rather than a second one to keep in step with it.
22. **`reconnect` is new, and jittered per stream** — the door emits `retry:` at open with a value
    drawn from the base ±33%, so a drained fleet does not reconnect in lockstep. `rule:programs/memory-priority`'s
    ordering puts this at priority 3, and it costs one field and one draw.
23. **Drain and reload close by ending the body.** SSE has no close frame and needs none: a clean end
    of the chunked body is the close, and the client's own reconnect is the recovery. An open stream
    keeps the compiled unit it began with, as ADR 0083 § 7 already requires of a connection.

## Stage 6 — the rulebook and the record

Prose, after the behaviour is green.

24. **One new record and no other number** — the two doors, `receive()`'s amendment of § 5, the
    framing refusals, the derived heartbeat, the jittered reconnect, the emitted proxy header, and
    the `Last-Event-ID` position. The number is claimed by the file that lands.
25. **`rule:tooling/echo-always-has-a-sink` gains its sixth row**: a connection isolate, and a request
    whose body is a stream, both write to that run's captured output with the `Cli\Text` carrier.
26. **`rule:security/response-body-is-one-typed-member`'s "five typed members" becomes seven**, with
    the two new ones classified — the content type of `Core\Response::stream` is a sink under
    `rule:security/sink-predicate`, and an event stream's `$data` is contagious where its `$event` and
    `$id` are sinks.
27. **`rule:concurrency/two-doors-one-isolate`** says the event stream's isolate waits on topics and
    not on a peer; **`rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`** says both
    spellings now exist and names them; **`rule:concurrency/connection-bounds-are-finite`** carries the
    derived heartbeat, the jittered reconnect and the unarmed `idle`.
28. **The spec rows**, regenerated: five new members and two new classes.

## Standing decisions

Settled with the user before the run. **None of these is re-opened by a session**; where one turns out
to be wrong in implementation, the fallback is named here and the record from stage 6 is where the
change is written down.

- **`receive()` is topics-only, and § 5's "no receive" means "no peer".** An SSE client cannot send on
  the stream, so there is no second source; the honest reading of § 5 is about the peer and not about
  the wait. Without this an event stream can only poll, which is the thing SSE exists to avoid. *No
  fallback: the goal is not worth running without it.*
- **One `Core\Sse` type across both doors, with honest refusals.** `receive()` throws `LogicError` in
  a streaming response. A narrower writer type for door two would remove the throw and break the
  reason one type is worth having — a helper taking `Core\Sse` written once and used from both sides.
  It is the shape `Core\Socket::current` already has outside a connection. *Fallback if the two
  lifetimes force different state: split the type and say so in the record.*
- **`send(mixed $data, …)`: a `string` goes out raw, anything else is JSON-encoded.** *Rejected:*
  a `send`/`sendJson` split on `Core\Socket::send`/`sendBytes`'s precedent — more consistent with the
  sibling class, but it doubles the surface for a distinction the caller never has to make, and
  `send($order, event: "order")` is the line people write. Record the rejection in the new record.
- **`$data` accepts `tainted`; `$event` and `$id` refuse it.** Framing belongs to us or to the
  serializer and a payload cannot escape a `data:` line once stage 2 normalizes it — the argument
  `Core\Response::json` already makes. `$event` and `$id` are `rule:security/unclassified-parameter-refuses-tainted`
  sinks: a client dispatches on the event name, so an attacker-chosen one is a live cross-tenant
  hazard, exactly as a tainted topic name is under ADR 0083 § 4.
- **Three refusals, all `LogicError`.** An `$event` or `$id` carrying `\n`, `\r` or NUL — stripping
  would silently change an event's name, and a NUL makes the client discard the id outright. An empty
  `$data` — the client provably does not dispatch an event with an empty data buffer, so it is a send
  that cannot arrive. And **`setStatus` on a path that opens an event stream**: an event stream is 200
  by protocol, and refusing the contradiction is what the both-cells-filled `500` already does rather
  than picking a winner. `Core\Response::stream` takes any status; the refusal is SSE's alone.
- **No replay buffer.** Resumption is `Last-Event-ID`, read off the request by the handler and passed
  through `args:` — the connection isolate shares nothing with its request and *cannot* read the
  header, which is `rule:security/isolate-shares-nothing` working as intended. Replay needs the
  application's own event log; a runtime ring buffer would be a bounded lie about durability. Door
  two reads the header directly, being in the request isolate.
- **`X-Accel-Buffering: no` is emitted**, beside `Cache-Control: no-cache, no-transform`. Production
  is a proxied origin by definition (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`),
  nginx buffers proxied responses by default, and that one default breaks SSE completely. This is an
  instruction *to* a proxy and not a proxy feature implemented here, so that rule's closed list of
  absences is untouched. Caddy, HAProxy and Envoy need nothing.
- **A request-scoped event stream is reconnected by `EventSource`, and that is documented, not
  fought.** When the request ends the browser reconnects; door two is for `fetch`-based readers and
  for progress UIs that close themselves, and a handler that wants the client to stop answers a
  non-200 on the next request. This is what keeps the two doors distinct rather than two spellings of
  one job, and it belongs in both members' docs.
- **`Core\Response::stream` is in this goal**, not split out. It is the same cell, the same head-early
  path in `serve.rs` and the same compile-time body-writer row; landing it separately would open
  stage 3's `serve.rs` work twice.
- **What this spends**, per `rule:programs/memory-priority`'s ordering, and it is stated because a
  member that spends must say so: one wake pair and one chunk in flight per open stream, plus the
  framing buffer and the subscriber queue a connection already pays for — O(in-flight), never O(events
  sent). Non-streaming responses are untouched: `Answer`'s whole variant keeps its exact `size_hint`
  and its `Content-Length`. The real cost is priority 4, simplicity: `Answer` becomes two-valued and
  every response path reads it, and `serve.rs` gains a path where the head is answered before the
  isolate ends. Both are the mechanical consequence of the feature and `body.rs` already pays the same
  price in the other direction.
