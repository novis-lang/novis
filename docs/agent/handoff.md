# Handoff

## State

**Goal `event-streams` — stages 0, 1 and 2 are landed, and stage 3's surface half is now half of
it too.** A Novis program can open a response body written over time and put chunks on it; nothing
in the server offers the cell yet, so every served request still answers whole.

**The member is `Core\Response::stream(string $contentType): Core\Response\Stream`**, with that
class's one member `write(string|bytes $chunk): void`. `$contentType` is a sink and is refused by
`spellable` on both sides of the bound; `$chunk` carries no classification, because a union cannot
— see the new playbook bullet, and the backlog item that owes it a record.

**The seam is `nvs_runtime::stream::BodySlot`** (`crates/nvs-runtime/src/stream.rs:300`): the cell
carries the connection's send timeout, `open()` answers the `Emit` and leaves a
`stream::Opened { content_type, drain }` for whoever frames the response. `Inbound` offers it
(`offer_response_stream`/`response_stream_slot`) and `Ctx` holds the writing half
(`set_body_stream`/`body_stream`, `crates/nvs-runtime/src/ctx/output.rs:496`).

**Off a connection there is no cell and the member degrades to `Ctx::write_output`** — gap 1's inert
declaration applied to the bytes as well. That is what makes a streamed body assertable from a
`.nvst` case, and the three cases under `tests/conformance/core/a-response-stream-*` are it.

**`Core\Response\Stream` is the second contextual class** — members, no slots, state on the `Ctx` —
listed beside `Core\Socket` in `a_class_with_slots_has_instance_members_and_the_reverse`. `"stream"`
is a body-writer row in `nvs_types::response::BODY_MEMBERS`, so `echo` beside it is `E0801`.

**Record `docs/decisions/0176.md` is still open** with the stage 0 correction and nothing else.

## Next group

**Stage 3: door two, the head-early path** — one file set: `serve.rs` and the two places a
per-request cell is made and attached. This is goal prose stage 3 item 12, the largest single change
in the goal, and it is what the earliest failing acceptance check names.

- [ ] **Offer the streaming-body cell to every request the server runs.**
      `nvs_runtime::stream::BodySlot::new(bounds.send)` beside the SSE cell at
      `crates/nvs-server/src/serve.rs:835`, attached through `Inbound::offer_response_stream` beside
      `offer_sse` at `crates/nvs-host/src/isolate.rs:280` —
      `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`.
- [ ] **Answer the head while the isolate is still running, and join it when the body ends.**
      `Reply::Run` is awaited to completion at `crates/nvs-server/src/serve.rs:859` and `answer`
      builds the response out of a `Completion` at `crates/nvs-server/src/serve.rs:1163`; a filled
      cell means the head goes out first over `Answer::Streaming`
      (`crates/nvs-server/src/serve.rs:153`). **Decide there what else the head carries**: `Opened`
      holds only a content type today, while `setStatus` and `setHeader` still cross on the
      `Completion`, which is built after the body has ended.
- [ ] **A streaming request is bounded by its own ceiling and not by `bounds::Connection`** — goal
      prose stage 3 item 13, over the same await at `crates/nvs-server/src/serve.rs:859`.

## Backlog

- `Core\Sse::stream(): Core\Sse` (goal prose stage 3 item 10) needs `current` and `send` first, and
  `sse::Event::frame` lives in `crates/nvs-server/src/sse.rs`, which `nvs-stdlib` cannot depend on —
  decide whether the framing moves to `nvs-runtime` or the member reaches it through the host seam.
- `two_body_writers_on_one_response_is_a_diagnostic` (`docs/agent/loop-goal.toml` stage 3) is a
  *new* refusal `nvs_types::response`'s module doc declines on purpose today, so it owes a decision.
- `write`'s union refuses a `tainted` chunk where `Core\Response::text` accepts one — the safe
  direction, taken deliberately; record it or split the member in stage 6.
- `docs/decisions/0176.md` is open with the stage 0 correction only; stage 6 names the rules it
  modifies on both sides.
