# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 11 are built. Nothing is blocked.

**The trace path is end to end.** A sampled request's events leave its isolate on
`nvs_runtime::host::Completion::trace` (`crates/nvs-runtime/src/host.rs:473`), taken there by
`Ctx::take_sampled_trace` (`crates/nvs-runtime/src/ctx/trace.rs:332`) under
`TraceContext::sampled` alone, so an unsampled request pays a load and a branch.
`nvs_server::trace::record` (`crates/nvs-server/src/trace.rs:255`) is the one place a finished run
reaches the exporter: the door calls it for a buffered answer and for a streamed one
(`crates/nvs-server/src/serve.rs:1307`, `:715`), and the ticker calls it for a fire
(`crates/nvs-server/src/schedule.rs:775`), whose head draw is
`Isolate::recording` (`crates/nvs-host/src/isolate.rs:317`). `otlp.rs`'s known gap 1 is closed.

**What a collector receives today is one span per sampled request**, which is known gap 1 in
`crates/nvs-server/src/trace.rs`'s module doc: a `query`, an `http` and a `spawn` event is filed only
under `DebugFlags::TRACE`, a served request never sets that bit, and setting it would file a `call`
event per call site — the cost `rule:observability/a-call-never-becomes-a-span` refuses. The gate that
separates the two is unbuilt and still belongs to this goal.

## Next group

**Stage 12: the served path, end to end** — one file set: `crates/nvs-cli/src/serve.rs` and
`crates/nvs-server/src/serve.rs`. The in-process serve fixture at
`crates/nvs-cli/src/serve.rs:3353` is the harness all three reuse; the tests are named by the stage 12
`[[check]]` in `docs/agent/loop-goal.toml` and must carry those exact names.

- [ ] **A runaway under `nvs serve` is a fatal and its core answers the next request**
      (`rule:errors/on-limit`). Two cases, `a_served_while_true_is_ended_as_a_fatal_and_the_core_answers_the_next_request`
      and `a_served_allocation_loop_is_ended_as_a_fatal_and_the_core_answers_the_next_request`: a
      request that spins and one that allocates are each ended by the tree's own ceiling, and the
      worker that ran them serves the request after. The watchdog registration the door hands over is
      `crates/nvs-cli/src/serve.rs:967`, and the ceilings are read at
      `crates/nvs-cli/src/serve.rs:955`.
- [ ] **A revalidation that fails to compile fails only the requests that resolve it afterwards**
      (`rule:config/an-edit-reaches-the-next-request-without-a-restart`, and M7's acceptance paragraph
      names it). `a_revalidation_that_fails_to_compile_fails_only_the_requests_that_resolve_it_afterwards`:
      a request already holding the old unit runs to completion while the next resolve refuses.
      The resolve is `crates/nvs-cli/src/serve.rs:955`'s program.

## Backlog

- The gate that files a `query`, an `http` and a `spawn` for a **sampled** request without turning on
  `DebugFlags::TRACE`'s per-call probes — `crates/nvs-server/src/trace.rs`'s known gap 1, owner this
  goal. It is a decision about where the bit lives, so it wants a record.
- A span carries no timestamps, so every trace lands as an instant — `crates/nvs-server/src/otlp.rs`
  known gap 1, owner M10.
- A fire's context carries no configuration of its own — `crates/nvs-server/src/schedule.rs` known
  gap 1, owner this goal.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30` and `crates/nvs-server/src/bounds.rs:62`,
  owner goal `unowned-closures`.
