# Handoff

## State

**Goal `m5-proofs` (M5). Stage 5 is closed — the record and its rule are on disk — and stage 6, the
options, is the earliest red check.**

[ADR 0184](../decisions/0184.md) decides the mechanism behind `on: "worker"` and creates one `designed`
rule, `rule:concurrency/on-worker-runs-the-child-on-another-core`, which the check at
`docs/agent/loop-goal.toml:9812` now resolves. The record argues only the mechanism, as § *Standing
decisions* fixed it: the child is **started** on another core (never migrated), through that core's
inbox; the answer comes back as a copy in a `RemoteWake`'s slot, the shape
`crates/nvs-host/src/blocking.rs:14-20` already uses in the other direction; the child stays its
parent's — cancelled with it, charged to its tree; under `nvs serve` the core is a sibling serving core
round-robin, elsewhere a lazily started worker core bounded at the core count. The standing decision's
starvation fallback is the record's § *Revisiting* trigger rather than a second sentence in the rule.

The rule sits third in `docs/rules/concurrency.json`, after
`rule:concurrency/a-child-belongs-to-the-calling-task` — the rule that says whose child it is comes
before the one that says which core runs it. `guardedBy` is empty on purpose; stage 9 fills it and flips
the status.

Stage 4's gap is unchanged and still in `## Backlog`. Nothing is blocked.

`[context]` gap closed here: stage 5's `shapes` did not name *Where a rule sits in the order*, so the
position of a new JSON entry had to be fetched by hand; it is in the list now. Still missing and not
fixed: `[context] adrs = ["0006"]` slices only *In short*, while the record needed
`docs/decisions/0006.md:151-153` and `:328-329`.

## Next group

**Stage 6: the options** — one file set: `crates/nvs-types/src/expr/isolate.rs`,
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-stdlib/src/script.rs`, `crates/nvs-host/src/group.rs`,
`crates/nvs-host/src/isolate.rs`, a new module beside them for the worker cores, and
`tests/conformance/isolate/`.

- [ ] **The three options are accepted and typed, and an unknown placement is refused** —
      `docs/agent/loop-goal.toml:9826` wants `spawn_script_accepts_limits_grants_and_on_and_checks_their_types`
      and `an_unknown_placement_for_on_is_refused_at_compile_time` under `-p nvs-types`. The option keys
      are already spelled at `crates/nvs-types/src/expr/isolate.rs:387`; `on:` accepts `"worker"` and
      `"here"` and nothing else, refused where that check is written, per
      `rule:concurrency/on-worker-runs-the-child-on-another-core` § 1 and
      [0184 § *Diagnostics*](../decisions/0184.md), which claims no code so the band is this slice's to
      pick.
- [ ] **The placement reaches the host seam** — lowering passes it as one more argument to
      `nvs_core_script_spawn` (`crates/nvs-ir/src/lower/expr.rs:3188`,
      `crates/nvs-stdlib/src/script.rs:619`), and `crates/nvs-host/src/group.rs:190` routes a worker
      placement to the new mechanism instead of starting every isolate on the calling core.
      `rule:security/isolate-values-cross-by-copy` is the crossing; the move is not available across
      cores.
- [ ] **The worker cores and their inbox** — a new module under `crates/nvs-host/src/`, built on
      `Worker::spawn` (`crates/nvs-host/src/lib.rs:180`) and `RemoteWake`
      (`crates/nvs-host/src/reactor.rs:219`). `docs/agent/loop-goal.toml:9836` wants
      `a_worker_child_runs_on_another_core_and_its_answer_is_copied_back`,
      `a_cancelled_parent_cancels_its_worker_child_across_cores` and
      `worker_cores_are_started_lazily_and_bounded_by_the_core_count` under `-p nvs-host`;
      `rule:concurrency/on-worker-runs-the-child-on-another-core` § 3–5 specifies all three.
- [ ] **The three `.nvst` cases** — `docs/agent/loop-goal.toml:9850` names
      `a-child-on-a-worker-core-answers-as-one-on-this-core-does.nvst`,
      `a-child-given-grants-holds-only-those-and-cannot-widen-them.nvst` and
      `a-child-given-limits-is-stopped-at-its-own-ceiling.nvst` under `tests/conformance/isolate/`.
      `limits:` never widens the tree's ceilings (`rule:security/isolate-budget-is-the-trees`).

## Backlog

- `rule:observability/spawn-is-its-own-event` says the child's wall time "already arrives on
  `ScriptResult`", and that shape's fields (`crates/nvs-stdlib/src/script.rs:428`) do not carry it — the
  overhead split reads it off the native `Completion` instead. Either the rule's sentence or the shape.
- `[context] adrs = ["0006"]` gives *In short* only; a session needing § *Decision* slices it by hand.
- `Core\Server`'s request-reading members and `traceId()` are `crates/nvs-stdlib/src/server.rs:11-14`'s
  known gap, not this goal's.
- `docs/plan/m5.md`'s stale prose, the `callable`-in-`Task::all` sentence included — goal `plan-truth`'s.
- Module-doc gaps tagged `unowned` in these files — goal `unowned-closures`'s.
