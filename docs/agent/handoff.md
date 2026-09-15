# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 11 are built. Stage 12 is half landed. Nothing is blocked.

**The served path now has a memory ceiling.** `Ctx::isolate`
(`crates/nvs-runtime/src/ctx/isolate.rs:364`) armed no threshold and set no ceiling, so every request
served over a socket — each one an isolate — allocated unbounded under any `[limits] memory`. It now
crosses what remains of the tree's ceiling through `set_memory_limit`, which is the number and the
setter `PlacedIsolate::build` (`:806`) already handed a child placed on another core, so the two
halves of `rule:security/isolate-budget-is-the-trees` agree. Two cases asserted the old shape as the
design and now assert the rule instead — `crates/nvs-runtime/tests/allocator_ceiling.rs:161` and
`crates/nvs-server/src/schedule.rs:1327`. `crates/nvs-cli/src/serve.rs:4139` is the case that holds
it end to end: a served doubling loop is a `500` and the same core answers the request after it.

**The CPU half of stage 12 is written, diagnosed and not landed**, because it exposes a second
defect whose fix is a design call this session had no room left to make well. A request stopped by
`rule:errors/on-limit`'s CPU ceiling leaves `SafepointFlags::CPU_LIMIT` standing in the word its
*tree* shares — which on the served path is the **connection's**, since `Ctx::isolate` calls
`share_safepoint_with` (`crates/nvs-runtime/src/ctx/safepoint.rs:59`) — and the safepoint's tail
lowers only `COLLECT | DEBUG_BREAK | MEMORY_LIMIT`
(`crates/nvs-runtime/src/ctx/safepoint.rs:475`). The watchdog expires the tree's deadline beside
raising the flag (`crates/nvs-runtime/src/ctx/safepoint.rs:301`), and `SafepointView` can raise and
expire but neither lower nor un-expire (`crates/nvs-runtime/src/ctx/safepoint.rs:279`). So the next
request on that keep-alive connection is a `500` at its first poll. Measured through the fixture
below, not inferred: it answered `500` and then `500`.

## Next group

**Stage 12: the served path, end to end** — one file set: `crates/nvs-runtime/src/ctx/safepoint.rs`,
`crates/nvs-host/src/isolate.rs` and `crates/nvs-cli/src/serve.rs`. The in-process serve fixture the
remaining cases reuse is `a_runaway_then_an_answer` at `crates/nvs-cli/src/serve.rs:3991`; it serves
two requests over one connection on one core through `serve_on_this_core` and returns both status
lines.

- [ ] **A stopped request gives the connection its safepoint word back**
      (`rule:errors/on-limit`). Decide which seam owns the clearing — the `Unpublished` guard that
      already clears the publication (`crates/nvs-host/src/isolate.rs:766`), or the safepoint tail
      (`crates/nvs-runtime/src/ctx/safepoint.rs:475`) — and what an expired deadline means to the
      connection that outlives the request that expired it. The alternative to weigh is a
      per-request word rather than a per-tree one, which `Ctx::share_safepoint_with`
      (`crates/nvs-runtime/src/ctx/safepoint.rs:59`) is the whole of. Write the reasoning where the
      rule does not already hold it.
- [ ] **A runaway `while (true)` under `nvs serve` is a fatal and its core answers the next
      request** (`rule:errors/on-limit`), as
      `a_served_while_true_is_ended_as_a_fatal_and_the_core_answers_the_next_request` beside the
      allocation case at `crates/nvs-cli/src/serve.rs:4139`. The tree is
      `[limits] cpu_time = "200ms"`, the program is the corpus's own spelling in
      `tests/conformance/error/a-loop-that-allocates-nothing-is-stopped-by-the-cpu-ceiling.nvst`,
      and the case skips where `nvs_host::cpuclock::no_ceiling_note` answers `Some`. The fixture's
      watchdog is already registered the way the door registers one
      (`crates/nvs-cli/src/serve.rs:4059`).
- [ ] **A revalidation that fails to compile fails only the requests that resolve it afterwards**
      (`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`), over the same fixture at
      `crates/nvs-cli/src/serve.rs:3991`.

## Backlog

- `Ctx::isolate` carries no `output_limit` and no `cpu_limit` across, so a served request is under
  no `[limits] max_output` — `crates/nvs-runtime/src/ctx/isolate.rs:364`, and
  `PlacedIsolate::build` at `:806` is the shape that is already right.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30`, `bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
- Stale prose in `docs/plan/m7.md`'s carrier list and `crates/nvs-cli/src/serve.rs:79-90` — goal
  `plan-truth`.
