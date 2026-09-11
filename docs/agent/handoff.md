# Handoff

## State

**Goal `event-streams` — stages 0, 1 and 2 are landed, and stage 3's `Core\Response::stream` half is
complete end to end.** A served request is offered `nvs_runtime::stream::BodySlot`, a program that
opens one is answered its head while it is still running, and the connection joins it when the body
ends. Goal prose stage 3 items 9, 11, 12 and 13 are done; **item 10, `Core\Sse::stream()`, is what is
left of the stage**, and it is what the stage's other two acceptance checks are waiting for.

**The head is everything the response declares.** `Core\Response::stream` takes the status and the
headers off the context and puts them on `stream::Opened`, because the head is on the wire from that
call — a declaration made afterwards reaches nothing. `crates/nvs-server/src/serve.rs`'s `streamed` is
the one place that head is written, `answer` being the whole-bodied twin beside it.

**A streamed request is still in flight.** Its isolate, its `crate::body::Supply` and its place under
`rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`'s tier C all move onto the
connection as `serve::Streamed`, and `serve::joined_when_ended` collects all three once the isolate
ends. `nvs_runtime::Ctx::take_body_stream`, called from `nvs_host::isolate::finish`, is what makes
"the body ends when the isolate does" a fact rather than a hope.

**Record `docs/decisions/0176.md` is still open** with the stage 0 correction and nothing else.

## Next group

**Stage 3: door two, the event-stream spelling** — one file set: `crates/nvs-stdlib/src/sse.rs`,
`crates/nvs-types/src/response.rs` and `tests/conformance/core/`. This is goal prose stage 3 item 10,
and it is the whole of what the stage's two red acceptance checks name.

- [ ] **`Core\Sse::stream(): Core\Sse`** — the five edits, all in `crates/nvs-stdlib/src/sse.rs`: a row
      beside `upgrade`'s at `crates/nvs-stdlib/src/sse.rs:66`, a body modelled on
      `crates/nvs-stdlib/src/sse.rs:150` that opens the request's cell at `text/event-stream` exactly as
      `crates/nvs-stdlib/src/response.rs:1581` opens it, and the `address()` arm.
      `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`. The standing decision that
      `setStatus` on this path is refused is the goal's own § *Standing decisions*, and the refusal is
      SSE's alone — `Core\Response::stream` takes any status and now carries it.
- [ ] **Both stream members are body writers.** `crates/nvs-types/src/response.rs:77` is the row list
      and `crates/nvs-types/src/response.rs:156` is the gate that reads it — check what that gate does
      with a class other than `Core\Response` before adding a row, since `Core\Sse::stream` is a member
      of a different class. `rule:security/response-body-is-one-typed-member`, and the check names
      `echo_beside_an_event_stream_is_a_diagnostic` and `two_body_writers_on_one_response_is_a_diagnostic`.
- [ ] **`tests/conformance/core/an-event-stream-that-ends-with-its-request-frames-every-event.nvst`** —
      the stage's third check, and it needs a `send` to write an event with. That is goal prose stage 4
      item 15; pulling it forward is the cheap order, the framing already existing at
      `crates/nvs-server/src/sse.rs:113`. Not checked: which crate a `Core\Sse->send` can reach that
      framing from — `nvs-stdlib` does not depend on `nvs-server`.

## Backlog

- `docs/decisions/0176.md` owes the stage 0 correction and the union-taint note — goal prose stage 6.
- `Core\Response::stream`'s reference card does not say the head goes out at the call, so a `setHeader`
  after it reaches nothing — `crates/nvs-stdlib/src/response.rs`, `rule:core-api/reference-card`.
- A streamed body is written under `Phase::Write`'s wait (`crates/nvs-server/src/io.rs`), so a
  long-lived stream meets a connection wait sized for writing a whole answer — goal prose stage 5.
- `bounds::Connection` is still `default()` everywhere; no `[server]` key overrides one —
  `crates/nvs-server/src/bounds.rs` § *Known gap*.
