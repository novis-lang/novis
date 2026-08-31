# Handoff

## State

**Stage 5 is one test from closed.** Its `nvs-stdlib` check names six tests and five are green;
only `a_socket_read_runs_on_the_reactor_and_parks_its_coroutine` is missing.

**A request carries a trace id, and it is the runtime's.** `nvs_runtime::trace_context` is a new
module — `TraceContext`, with `started`, `continuing` (an inbound `traceparent` adopted whole, a
malformed one starting a new trace rather than throwing, per ADR 0076 § 2), `traceparent` rendering
and the three accessors. `Ctx` holds one eagerly, drawn in `Ctx::new`, with `set_trace_context` for
the inbound path goal 6 will write; that module's doc is the home of why it is eager, why it is not
ADR 0018's per-call trace, and what it spends. `nvs-runtime` gained `rand` for the draw, which adds
no crate to the tree.

**`Core\Http\Client` propagates it.** `http::traceparent_of` reads `[trace] propagate` (§ 6 ships it
**on**; off is the word `false` and nothing else), `Call::traceparent` carries the answer, and
`compose` emits it — skipping its own where the caller already wrote a `traceparent` header, because
two of them is what the W3C format tells a receiver to read as none.

**The driver's acceptance failure on `examples/http.nvs` is still the driver, not the tree.**
`local_origin` is on disk at `tools/loop.py:1125`; the running process imported that module before it
existed, so its sweep serves nothing on 8099. Restarting the run is the whole fix — nothing in this
tree changes it.

## Next group

**The stage-5 closer, then stage 6 opens.** Item 1 is its own file set —
`crates/nvs-stdlib/src/http/transport.rs`, the file this session just left — and items 2-3 share
`crates/nvs-stdlib/src/cache.rs` (new) with `crates/nvs-stdlib/src/registry.rs`. Take 1 first: it
closes a stage.

- [ ] **`a_socket_read_runs_on_the_reactor_and_parks_its_coroutine`** — ADR 0051's "over the
      runtime's own reactor, not a second event loop". The check's `args` is `-p nvs-stdlib`, so it
      hosts in `crates/nvs-stdlib/src/http/transport.rs:562`'s `mod tests` and not in `nvs-host`.
      What it asserts is already observable: `crates/nvs-host/src/net.rs:184`'s
      `NvsStream::is_parked_on`, with `crates/nvs-host/src/net.rs:1054`'s
      `a_socket_read_that_would_block_parks_its_coroutine` as the shape to drive it from a coroutine.
- [ ] **`Core\Cache::local`** — ADR 0059 §§ 1-3: two members with separate contracts, an entry that
      may be absent at any time, and a per-core tier charged to the core and capped. Rows, cards,
      bodies and the `address()` arm in a new `crates/nvs-stdlib/src/cache.rs`, registered at
      `crates/nvs-stdlib/src/registry.rs:1054`.
- [ ] **`Core\Cache::shared`, over goal 2's graph copy** — ADR 0059 § 2: a put and a get use the same
      walk the isolate boundary does, never a third mechanism. Same two files —
      `crates/nvs-stdlib/src/cache.rs` and `crates/nvs-stdlib/src/registry.rs:1054` — over
      `crates/nvs-runtime/src/graph.rs:672`'s `encode` and `crates/nvs-runtime/src/graph.rs:869`'s
      `decode`, which are the walk.

## Backlog
- `examples/cache.nvs`'s five frozen lines close stage 6 — `docs/agent/loop-goal.toml`, stage 6.
- `Core\RateLimit`'s five tests, GCRA in both tiers — ADR 0075 §§ 2, 4, 5.
- Redis needs a container; the compose file goal 5 uses is the same one — plan § *Blocking*.
- ADR 0076 § 6's `[log]` record gaining `trace_id`/`span_id` has a source now, and no writer:
  `Core\Log` is stage 7's.
- Head-based `[trace] sample` is unread, so a root trace's flag is always `00` —
  `crates/nvs-runtime/src/trace_context.rs`'s module doc says what will set it.
