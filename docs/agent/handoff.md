# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
stages 3 to 11 are built, and stage 12 has two of its three cases green. Nothing is blocked.

**A served request is now its own request tree.** `Ctx::reroot`
(`crates/nvs-runtime/src/ctx/safepoint.rs:96`) mints a safepoint word and a deadline nothing else
polls, and `serve_connection` calls it in front of every request
(`crates/nvs-server/src/serve.rs:1229`) — ahead of the `set_config` that arms the ceilings off them,
for the ordering `Ctx::isolate` already keeps. Both words were the *connection's* before it: a
request stopped at `rule:errors/on-limit`'s CPU ceiling left `SafepointFlags::CPU_LIMIT` standing
with the deadline expired beside it, and the next request down the same keep-alive connection was a
`500` at its first poll. The pair is **replaced rather than cleared**, and the method's doc is the
home of why — `SafepointView::expire_deadline` is one-way by design, and no seam that watches an
isolate end knows whether its tree ended. The allocation is reused where the context is the tree's
only holder, which is the same fact that keeps
`rule:concurrency/after-response-outlives-the-connection`'s deferred work stopped.

**Stage 12's CPU case is landed**: `a_served_while_true_is_ended_as_a_fatal_and_the_core_answers_the_next_request`
(`crates/nvs-cli/src/serve.rs:4145`) serves a spinning entry and then an answering one over one
connection on one core through `a_runaway_then_an_answer` (`:3991`). Measured both ways — with the
re-root ablated it answers `500` and then `500`.

**The stage-12 acceptance check stays red until the third case exists.** One `[[check]]` names all
three tests, and `a_revalidation_that_fails_to_compile_fails_only_the_requests_that_resolve_it_afterwards`
is not written yet. That is the next group, and it is the whole of what stands between this goal and
its last stage.

## Next group

**Stage 12: the served path, end to end** — one file set: `crates/nvs-cli/src/script.rs`. The
compiler under test lives there; `revalidating()` (`crates/nvs-cli/src/script.rs:954`) and
`checking(validate, freq)` (`:961`) build one, `a_file_running` (`:933`) writes an entry, and
`a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile` (`:1293`) is the sibling
whose shape both cases below take.

- [ ] **A revalidation that fails to compile fails only the requests that resolve it afterwards**
      (`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`, first paragraph). The test the
      stage-12 `[[check]]` names by exactly that name, so it goes in `nvs-cli`. A request that
      resolved the last good content runs to completion, the broken resolution is what *this* caller
      gets as checked-return data, and the path's pointer is left naming the content that compiled —
      `Compiler::record` (`crates/nvs-cli/src/script.rs:621`) and `Compiler::advance` (`:598`) are
      the two halves, and `a_stale_revalidation_does_not_overwrite_a_fresher_published_one` (`:1400`)
      already asserts the pointer one.
- [ ] **A storm against a broken file costs one compile and one rendering of its spans**
      (same rule, first paragraph's tail). The `Failed` entry is keyed by content, so every later
      request on the same broken bytes is answered from the table rather than compiled again;
      `the_compile_counter_counts_compiles_and_not_cores` (`crates/nvs-cli/src/script.rs:1446`) is
      the counter to assert it against.

## Backlog

- An upgraded connection's isolate is built from the connection's context and so joins the tree of
  the request that upgraded it, where `rule:concurrency/a-connection-is-a-root-isolate` makes it a
  root with a budget of its own — `crates/nvs-server/src/serve.rs:1560`.
- `Core\Metrics`'s three rows belong to goal `m8-stdlib-depth`; the `unowned` gaps at
  `crates/nvs-server/src/route.rs:30` and its siblings belong to goal `unowned-closures`.
- `docs/plan/m7.md`'s carrier list and the "one core" rows are goal `plan-truth`'s stale prose.
