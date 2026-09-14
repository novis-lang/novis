# Handoff

## State

**Goal `m5-proofs` (M5), stage 3's first two items landed; stage 1's floor is goal `m4-refusals`'s
list, carried.**

`Core\Session::start` now asks `rule:security/request-state-throws-in-an-isolate`'s question — is
there a request at all — before it asks anything about the store, so a CLI program, a job worker and a
child isolate are refused with `Core\Request`'s one `LogicError` instead of being handed the session a
configured deployment would have minted, cookie and all. That is the other ordering from the one the
last handoff proposed, and the commit says why; the five session cases that call `start` carry a
`--GET--` section now, and the two whose `--SKIPIF--` probed the store through `start` ask
`Core\Cache::shared` instead, since scaffolding answers no request whatever the case does.

The boundary itself is one `.nvst` case under `tests/conformance/isolate/`, with the parent answering
its own request throughout — a CLI case cannot tell the boundary from a process that has no request,
because every line of one refuses. The rule's fragment carries the `isDraining` sentence the goal's
§ *Standing decisions* settled.

The floor's one red check was the plan index's M4 cell, synced from the chain. Nothing is blocked.

## Next group

**Stage 3: the isolate proofs the Verify paragraph names, continued** — one file set:
`crates/nvs-host/tests/limits.rs`, both beside the depth breach it already holds.

- [ ] **A child's memory breach and CPU breach are each `ok = false` with the parent still running** —
      one test each beside the depth breach at `crates/nvs-host/tests/limits.rs:769`, reading the word
      the handler is handed out of `rule:errors/on-limit`'s report as that test does, rather than out
      of the message. `rule:security/isolate-failure-is-a-value`.
- [ ] **A cyclic argument crosses a real spawn** — driven through `Isolate::run` at
      `crates/nvs-host/src/isolate.rs:401` rather than over the walk alone, reading the cycle back out
      of the child's answer, beside `crates/nvs-host/tests/limits.rs:769`. The walk's own cyclic fixture
      is `crates/nvs-runtime/src/graph.rs:1127`, and the `table()`/`node()`/`borrow_object` helpers it
      builds the ring with are that module's private test ones — settle what an integration test can
      build a cycle out of before budgeting this. `rule:security/isolate-values-cross-by-copy`.

## Backlog

- `Core\Server`'s request-reading members — the request's environment and `traceId()` — are
  `crates/nvs-stdlib/src/server.rs`'s known gap, and not this goal's.
- The stale `callable`-in-`Task::all` prose of `docs/plan/m5.md` is goal `plan-truth`'s.
- Stage 3's remaining `.nvst` room is the `with(args:)` side of the boundary: no isolate case passes an
  argument at all, so every one of them spawns with the default.
