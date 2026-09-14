# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6's third item is on disk: the worker cores exist, with an inbox, a
bell and an answer that crosses back in a slot.**

`crates/nvs-host/src/worker.rs` is `rule:concurrency/on-worker-runs-the-child-on-another-core`'s
mechanism, built as ADR 0184 §§ 3–5 fix it. `worker::place` (`crates/nvs-host/src/worker.rs:516`)
is the blocking pool's handoff pointing the other way: take a `RemoteWake` for the running task,
post the work to another core's inbox, ring its bell, park; the far core runs it **as a task**, so
it may park and spawn children there. `Answering`'s `Drop` is the one path out — a return, a
contained panic and a forced unwind all answer, so no parent is left parked. Cores are started on
the first placement and bounded at `cpus().len().max(1)`, one thread per core.

**Built, not routed.** `SchedulerHost::start_isolate` still sends both placement words to this
core's own start, and the reason is now a shape rather than a missing mechanism: a
`nvs_runtime::script::Program` is a `Box<dyn FnOnce(&mut Ctx, Value) -> Value>` the *parent's*
resolver built, so it is not `Send` and means nothing on another core, and a `Value` is reachable
from one core by construction. `crates/nvs-host/src/group.rs`'s `# Known gaps` is that gap's one
home and says what the seam has to carry instead. `limits:` and `grants:` still stop at the checker.

## Next group

**Stage 6: the options, below the checker** — one file set: `crates/nvs-runtime/src/host.rs`,
`crates/nvs-host/src/group.rs`, `crates/nvs-host/src/worker.rs`, `crates/nvs-stdlib/src/script.rs`,
`crates/nvs-ir/src/lower/expr.rs` and `tests/conformance/isolate/`.

- [ ] **The seam names the child's program instead of carrying it** — `start_isolate`
      (`crates/nvs-runtime/src/host.rs:576`) takes a `Program` today, which a worker placement
      cannot cross with. Carry what the far core can resolve for itself — the path, or the class
      and method `Entry::Method` names — and the argument as `nvs_runtime::graph::encode`'s bytes
      (`crates/nvs-runtime/src/graph.rs:719`, decoded by `:917`), which is ADR 0184 § 2's copy at
      every node. The two call sites are `crates/nvs-stdlib/src/script.rs:661` and `:917`, which
      resolve before they reach the seam; `crates/nvs-host/src/group.rs:191` is the one implementor.
      `rule:security/isolate-values-cross-by-copy` is what may cross.
- [ ] **`Placement::Worker` routes to a worker core** — the arm at
      `crates/nvs-host/src/group.rs:211`, over `worker::place` at
      `crates/nvs-host/src/worker.rs:516`. It deletes `crates/nvs-host/src/group.rs`'s `# Known
      gaps` paragraph and the second half of `crates/nvs-host/src/worker.rs:86`'s.
      `rule:concurrency/on-worker-runs-the-child-on-another-core` is the rule.
- [ ] **`limits:` and `grants:` below the checker** — the lowering drops both today
      (`crates/nvs-ir/src/lower/expr.rs:3206` writes the placement beside the path, the `args:`
      value and the `output:` spelling and nothing else); `crates/nvs-stdlib/src/script.rs:606`
      is where `placement_of` reads its word and where the two others join it.
      `rule:security/isolate-budget-is-the-trees` is the sub-cap rule and
      `rule:concurrency/an-upgrades-options-are-spawn-scripts` the sibling site's wording.
- [ ] **The three `.nvst` cases** `docs/agent/loop-goal.toml:9880` names, under
      `tests/conformance/isolate/` — a worker child answering as a local one does, grants that
      only narrow, limits that stop a child at its own ceiling. They need the two items above.

## Backlog

- A serving core does not register its inbox, so `nvs serve` places on the lazily started cores —
  ADR 0184 § 5's pre-authorized fallback; `crates/nvs-host/src/worker.rs`'s `# Known gaps`.
- The `isDraining`-is-process-state sentence for `rule:security/request-state-throws-in-an-isolate`
  — the goal's § *Standing decisions* says where it goes if `rules.py` refuses it.
- Stage 7's bench and stage 8's sanitizer leg are untouched — `docs/agent/loop-goal.md`.
