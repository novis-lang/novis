# Handoff

## State

**Goal `event-streams` — stages 0, 1 and 2 are landed; stage 3 is landed except `Core\Sse::stream`.**
Both stream members are now compile-time body writers, so `echo` beside either, and two different
body writers on one response, are `E0801` before anything runs
(`rule:security/response-body-is-one-typed-member`). The stage's `nvs-types` acceptance check is
green. `Core\Response::stream` is complete end to end; `Core\Sse::stream` is the whole of what is
left, and it now has a named blocker rather than an open question.

**`Core\Sse::stream` cannot land before `Core\Sse->send`, and that is a fact about two gates, not a
preference.** A member answering `CoreTy::Instance(r"Core\Sse")` panics at run time while the class
has no instance member (`crates/nvs-stdlib/src/instance.rs:270`, and the playbook bullet), and the
conformance floor of three cannot be met by a handle nothing can be called on. The member was written
whole this session — row, card, body, `address()` arm, two `#[test]`s over the cell, one case — and
backed out rather than half-landed; `git log` holds nothing of it, so the next session writes it
again alongside `send`.

**The framing moved to `crates/nvs-runtime/src/sse.rs`** to unblock exactly that. Stage 2 put it in
`nvs-server`, which `nvs-stdlib` cannot name without closing a cycle, and `send`'s payload is a
`Core\Sse` member's. Nothing called it yet, so the move was the file plus two module lists; the
argument is in its own module doc and in the goal's stage 2 paragraph.

**Record `docs/decisions/0176.md` is still open** with the stage 0 correction, and now owes the
framing's crate as well.

## Next group

**Stage 3: door two, and it is one slice with stage 4's writer** — one file set:
`crates/nvs-stdlib/src/sse.rs`, `crates/nvs-runtime/src/sse.rs` and `tests/conformance/core/`. The
two members have to land together for the reason `## State` names, so the first two items are one
commit's worth of work in one file.

- [ ] **`Core\Sse->send(mixed $data, ?string $event = null, ?string $id = null): void`** — the
      instance member, and the one that makes the class instantiable at all. The framing is
      `crates/nvs-runtime/src/sse.rs:113`'s `Event`, now reachable; the writer to hand a frame to is
      `ctx.body_stream()`, whose whole-body twin is `crates/nvs-stdlib/src/response.rs:1668`. A
      `string` goes out raw and anything else is JSON-encoded, `$data` accepts `tainted` and
      `$event`/`$id` refuse it, and the three `LogicError` refusals are the goal's own § *Standing
      decisions* — all of that is settled there, so none of it is a call to make.
      `rule:concurrency/two-doors-one-isolate`.
- [ ] **`Core\Sse::stream(): Core\Sse`** — the five edits in `crates/nvs-stdlib/src/sse.rs`: a row
      beside `upgrade`'s at `crates/nvs-stdlib/src/sse.rs:66`, a body modelled on
      `crates/nvs-stdlib/src/sse.rs:150` that opens the request's cell at `text/event-stream` exactly
      as `crates/nvs-stdlib/src/response.rs:1581` opens it, and the `address()` arm. The status is
      left on the context rather than taken — an event stream is 200 by protocol, so there is nothing
      to carry out, and `setStatus` beside it is refused at compile time instead.
      `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`.
- [ ] **`tests/conformance/core/an-event-stream-that-ends-with-its-request-frames-every-event.nvst`**
      — the stage's third check, writable once `send` exists: off a connection the chunks reach the
      program's own output, which is how
      `tests/conformance/core/a-response-stream-writes-chunks-and-declares-its-own-type.nvst` asserts
      the sibling. Three cases, not one — `crates/nvs-stdlib/tests/conformance_coverage.rs:155` is a
      floor per member and `BELOW_THE_FLOOR` only shrinks.

## Backlog

- `tests/conformance/reject/set-status-on-a-path-that-opens-an-event-stream-is-refused.nvst` is the
  stage's last unwritten check and needs a diagnostic of its own — `crates/nvs-types/src/response.rs`.
- `docs/decisions/0176.md` owes the stage 0 correction, the union-taint note and the framing's crate
  — goal prose stage 6.
- `Core\Response::stream`'s reference card does not say the head goes out at the call, so a
  `setHeader` after it reaches nothing — `crates/nvs-stdlib/src/response.rs`, `rule:core-api/reference-card`.
- A streamed body is written under `Phase::Write`'s wait (`crates/nvs-server/src/io.rs`), so a
  long-lived stream meets a connection wait sized for writing a whole answer — goal prose stage 5.
- `bounds::Connection` is still `default()` everywhere; no `[server]` key overrides one —
  `crates/nvs-server/src/bounds.rs` § *Known gap*.
