# Handoff

## State

**Goal 6, Stage 5: the carrier's body is a stream, and nothing supplies one yet.**
`nvs_runtime::Inbound` gained a `RequestBody` — chunks pulled one at a time, each borrowed from the
supplier's own buffer and invalidated by the next pull, so no implementation may accumulate. That
trait's doc comment at `crates/nvs-runtime/src/ctx.rs:4556` is the one home of *why a reader and not
bytes*: ADR 0105 § 5's two caps measure different things and a buffering door collapses them into the
larger, which ADR 0106 § 13 then multiplies by `max_in_flight`. `Ctx::inbound_mut` is how a member
reaches it, beside `Ctx::inbound` for the head; `Inbound` lost `Clone`, which nothing used and which
a body makes wrong.

**Nothing supplies one, and that is a blocker rather than an omission.** `hyper`'s h1 dispatcher
polls read and write in one loop on one task, so anything that parks on the body from inside
`serve_connection`'s synchronous service closure deadlocks on the *first* chunk of the *smallest*
body. The comment at `crates/nvs-server/src/serve.rs:479` is the one home of that and of what closing
it takes; the playbook carries it as a trap.

**The failing acceptance check is still `examples/upload.nvs`**, and it stays failing until the
service future is real: `body()`, `bodyStream()` and `files()` all pull, so none of them can be
exercised over a socket before item 1 below. That is why this group re-orders the previous one rather
than continuing it.

**`[context]` gaps.** `adrs` selects no section of ADR 0105 — this item needed §§ 3, 5 and 6 and its
*Verification*, all peeked by hand — and no § 1 of ADR 0138, which is what says a park suspends the
task rather than the thread. Spec § 15 still has no `spec` selector.

## Next group

**The body's supply path, which is what every remaining ADR 0105 item waits on.** One file set:
`crates/nvs-server/src/serve.rs`, `crates/nvs-host/src/isolate.rs`, `crates/nvs-runtime/src/host.rs`.
Item 1 unblocks 2 and 3 and nothing else in the goal touches it.

- [ ] **The service future is real and the isolate runs as a peer task** — the enabling change, at
      `crates/nvs-server/src/serve.rs:479`, whose comment states the deadlock and the way out.
      `Isolate::start` already spawns a task (`crates/nvs-host/src/isolate.rs:414`); what parks is
      `Started::join`'s loop at `crates/nvs-host/src/isolate.rs:348`, so `Running`
      (`crates/nvs-runtime/src/host.rs:342`) needs a non-parking "is it done" beside `join`, and the
      service closure becomes an `async move` that answers `Pending` until it is. **The care is in
      cancellation, not in the happy path**: ADR 0072 § 4 says control does not leave with work still
      running, and today `Started::join` is what cancels the child and keeps parking until it has —
      a dropped service future must do the same or the rule is quietly gone.
- [ ] **The door supplies the body** — `crates/nvs-cli/src/serve.rs:312`'s carrier gets a
      `RequestBody` over `hyper`'s `Incoming`, pulled under `nvs_host::block_on`, and the same at
      `crates/nvs-server/src/serve.rs:1142`'s test door. ADR 0105 § 5's `upload_total` is counted
      here, on the wire, because that section says the server enforces it and a `Content-Length`
      already over it is refused before dispatch. The case that matters is a body larger than one
      socket read arriving in full.
- [ ] **`Core\Request::body()` and `bodyStream()`** — spec § 15, ADR 0105 § 3's three ways to
      consume, reading through `crates/nvs-runtime/src/ctx.rs:4334`'s `inbound_mut` from
      `crates/nvs-stdlib/src/request.rs:378`'s shape. § 8's exclusivity with `files()` is a flag on
      the carrier, not a second reader.

## Backlog

- `files()`, the lazy iterator — ADR 0105 §§ 1 and 2, and the failing `examples/upload.nvs` check.
- `[limits] request_body` and `upload_total` are not rows in `nvs-config` yet — ADR 0105 § 5.
- The door's `501` still has no end-to-end case — `crates/nvs-stdlib/src/request.rs`'s `//!`.
- `Core\Request` answers seven of spec § 15's fifteen — that module's `//!` owns the rest.
- ADR 0102's route table and `Core\Request::route()` — `docs/plan/m7.md`.
