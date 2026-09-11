# Handoff

## State

**Goal `event-streams` — stage 5 is landed: an event stream now has a bound table of its own.**
`crate::bounds::EventStream` (`crates/nvs-server/src/bounds.rs`) is what `Answer::Streaming` carries
for an event stream and `None` for every other body: the keep-alive, the lifetime that closes a
stream however busy it was, the drain period a shutting-down server gives it, and the reconnection
hint it opens with. `Connection::idle` is the one field it must not read, and the absence is stated
in that type's own doc.

**`Connection` has a new field**, `reconnect`, three seconds before the draw; `crate::bounds::reconnect_hint`
spreads it by up to a third either way, per stream, and `rand` is now a dependency of `nvs-server`.
`Connection::message` and `Connection::send` are read one layer down at `nvs_runtime::stream::open`,
which refuses a chunk over the message bound (`nvs_runtime::stream::CHUNK_TOO_LARGE`) **without**
closing the stream — a program handing over too much, not a peer.

Two renames a later reader will look for: `Answer::beating` is `Answer::as_an_event_stream` and takes
the whole `Connection` and the drain, and `Answer::Streaming`'s second field is `alive` rather than
`beat`. All nine tests of stage 5's three checks pass.

**`Core\Socket::receive` is still inside `Ctx::deliver`'s known gap**, and record
`docs/decisions/0176.md` is still open with the stage 0 correction and owes the framing's crate.

## Next group

**Stage 6: the rulebook and the record** — one file set: `docs/rules/` and `docs/decisions/`. Prose
only, over behaviour that is already green, so nothing here compiles anything.

- [ ] **The three bounds this goal added to the rule** — `docs/rules/concurrency/connection-bounds-are-finite.md:1`
      still lists the fields a WebSocket reads and nothing else. It owes the derived heartbeat
      (`crate::bounds::heartbeat`), the jittered reconnect (`crate::bounds::reconnect_hint`) and the
      **unarmed** `idle`, which is the one thing a list of bounds cannot show by listing.
      `rule:concurrency/connection-bounds-are-finite`.
- [ ] **The sink table's sixth row** — `docs/rules/tooling/echo-always-has-a-sink.md:6`, after the
      job-worker row: a connection isolate, and a request whose body is a stream, both write to that
      run's captured output with the `Cli\Text` carrier. `rule:tooling/echo-always-has-a-sink`.
- [ ] **Five typed members becomes seven** — `docs/rules/security/response-body-is-one-typed-member.md:1`,
      whose first sentence still counts five. The two new ones are classified as the goal's
      § *Standing decisions* argues them: `Core\Response::stream`'s content type is a sink, and an
      event stream's `$data` is contagious where its `$event` and `$id` are sinks.
      `rule:security/response-body-is-one-typed-member`.
- [ ] **One new record, `docs/decisions/0177.md`** — re-derive the next free number before creating
      it. The two doors, `receive()`'s amendment of ADR 0083 § 5, the framing refusals, the derived
      heartbeat, the jittered reconnect, the emitted `X-Accel-Buffering` and the `Last-Event-ID`
      position, with the rejected `send`/`sendJson` split recorded. The same pass closes
      `docs/decisions/0176.md:1`, which owes the stage 0 correction and the framing's crate.

## Backlog

- No `[server]` key reaches any `crate::bounds::Connection` field — that module's § *Known gap*.
- Stage 6's spec rows: five new members and two new classes — `nvs-stdlib`'s `spec_registry_coverage`.
- `Core\Socket::receive` inside `Ctx::deliver`'s known gap — that module's own doc.
- `docs/agent/carried-gaps.md` is where any of these goes if the goal switches first.
