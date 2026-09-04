# Handoff

## State

**Goal 6, M7. ADR 0072 §§ 6-7 have landed**, which was the group the last handoff named — though not
where it expected. A request is an isolate, and an isolate's context is **no longer sealed**
(`crates/nvs-runtime/src/ctx.rs:3164`): every program that runs as one has § 6's trigger, which is
the frame that produced the answer returning. `nvs_host::isolate`'s completion path is the drain
(`crates/nvs-host/src/isolate.rs:496`): it files the answer, drops the `Ended` guard so the joiner
may take it, **detaches the tree from the joiner** (`nvs_host::detach_current`), yields one slice so
the response goes out first, and only then runs `nvs_runtime::deferred::run_deferred`. The detach is
the load-bearing half — a task's death cancels what it left running, so a tree still parented to the
connection would be torn down by the very event § 6 says it must outlive.

**§ 7's cap counts trees, per core.** `Ctx::defer` now answers `Result<(), DeferError>`: `Sealed` is
§ 6's last bullet, `AtCapacity { cap }` is the new one, and `Core\Task::afterResponse` renders each
as its own `RuntimeError` at the call site. A tree takes one slot at its **first** registration and
gives it back at the end of the drain or as its context goes down — `crate::deferred::trees_in_flight`
is the reading. `[deferred] max_concurrent` defaults to 256, ADR 0072 § 7's own number.

**The two acceptance-check names moved**, to `-p nvs-host` and `-p nvs-runtime`, and the new blocks
in `docs/agent/loop-goal.toml` carry the reason: `crates/nvs-server` forbids `unsafe_code`, so no
test there can build the `callable` a registration is. The playbook bullet is the general shape.

**The server itself needed no change** beyond a doc paragraph on `Peer::collect` — the seam is one
layer below it.

## Next group

**ADR 0076's observability, over `crates/nvs-server` and `crates/nvs-runtime`'s trace context** —
what is left of the stage-6 check, whose other three names now pass. The file set is
`crates/nvs-server/src/serve.rs`, `crates/nvs-server/src/route.rs` and
`crates/nvs-runtime/src/trace_context.rs`.

- [ ] **A trace id exists for every request, sampled or not, and an inbound `traceparent` continues
      it** (ADR 0076 § 1) — `nvs_runtime::TraceContext` is at
      `crates/nvs-runtime/src/trace_context.rs:39` and a context answers it at
      `crates/nvs-runtime/src/ctx.rs:1824`; the door reads the arrived headers at
      `crates/nvs-server/src/serve.rs:571`, which is where the walk already reads them once. Tests:
      `an_inbound_traceparent_is_continued_and_a_missing_one_is_generated` and
      `a_trace_id_exists_whether_or_not_the_request_is_sampled`.
- [ ] **The exporter, and the `route` label reaching it** (ADR 0076 § 1) —
      `crates/nvs-server/src/route.rs:40` states "nothing exports the label yet" as known gap 2 and
      `crates/nvs-server/src/route.rs:115` is the `label` with no caller. Tests:
      `the_default_metric_series_are_exported` and `no_probe_is_added_to_the_measured_path`; the
      second is a claim about the *measured* path, so read § 1 for what may be added to it before
      writing either.

## Backlog

- `fleet` is still unarmed: ADR 0073 § 3's refusal, because no lease can be taken anywhere in this
  tree — the stage-6 check comment owns the triage and files it against ADR 0073 *Verification*'s M8
  bullet.
- A deferred closure may not touch `Core\Response` (ADR 0072 § 6, third bullet) — a compile-time
  diagnostic where the call is statically visible, a throw otherwise. Nothing enforces either today.
- A drain that is still running when a core stops loses its work with no record; § 6 says nothing is
  durable, so this is a note for whoever writes shutdown, not a gap — `docs/adr/0072` § 6.
- ADR 0105's raw-body gap, per the goal's § *Standing decisions*.
