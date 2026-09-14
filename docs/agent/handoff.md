# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6's second item is on disk: a spawn's `on:` crosses the host seam as
`nvs_runtime::host::Placement`, and the seam routes on it.**

The lowering writes the placement as a fourth argument beside the path, the `args:` value and the
`output:` spelling, materializing `"here"` where the program named none
(`crates/nvs-ir/src/lower/expr.rs:3206`). `nvs_core_script_spawn` takes four arguments and the method
form five, with the entry's parameter names moved to index 4 (`crates/nvs-stdlib/src/script.rs:642`,
`:820`). `placement_of` is fatal on any other word where `output_of` throws, because only the compiler
can produce one — `check_placement` refuses the rest at the spawn site
(`crates/nvs-stdlib/src/script.rs:606`).

**Routed, not placed.** `SchedulerHost::start_isolate` matches on the placement and both words reach
this core's own start (`crates/nvs-host/src/group.rs:211`), because the other core's inbox is not
built; `crates/nvs-host/src/group.rs`'s module doc `# Known gaps` is the one home of that. So a
program that writes `on: "worker"` today compiles, crosses the seam, and runs on its parent's core.
`limits:` and `grants:` still stop at the checker and are dropped by the lowering.

## Next group

**Stage 6: the options, below the checker** — one file set: `crates/nvs-host/src/` (a new `worker.rs`
beside `blocking.rs`, plus `group.rs`, `isolate.rs`, `reactor.rs`), `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-stdlib/src/script.rs` and `tests/conformance/isolate/`.

- [ ] **The worker cores and their inbox** — a new `crates/nvs-host/src/worker.rs`, built on the
      blocking pool's shape (`crates/nvs-host/src/blocking.rs:320` is the run-elsewhere-then-wake
      handoff, `:76` its bound, `:107` the pool): cores started lazily and bounded at the core count,
      a per-core inbox the reactor drains, and the answer copied into a slot whose
      `RemoteWake` ends the parent's park (`crates/nvs-host/src/reactor.rs:219`).
      `rule:concurrency/on-worker-runs-the-child-on-another-core` is the rule and ADR 0184 §§ 2–5 the
      mechanism; the `Placement::Worker` arm at `crates/nvs-host/src/group.rs:211` is where it plugs
      in, and group.rs's `# Known gaps` is the sentence it deletes. The three tests
      `docs/agent/loop-goal.toml:9849` names are this item's.
- [ ] **`limits:` and `grants:` below the checker** — the lowering drops both today
      (`crates/nvs-ir/src/lower/expr.rs:3164`), so they cross as two more arguments the way `on:` now
      does, the helper reads them at `crates/nvs-stdlib/src/script.rs:642`, and the isolate applies
      the sub-cap and the narrowed overlay at `crates/nvs-host/src/isolate.rs:162`. The key set is
      `SUB_CAP_SETTINGS`/`SUB_CAP_COUNTS` at `crates/nvs-types/src/expr/isolate.rs:414`;
      `rule:security/isolate-budget-is-the-trees` is what a sub-cap means and
      `rule:security/capability-check-at-the-door` where a grant is asked.
- [ ] **The three `.nvst` cases** `docs/agent/loop-goal.toml:9860` names, under
      `tests/conformance/isolate/`: a worker child answers as a local one does, a child given grants
      holds only those, a child given limits is stopped at its own ceiling. They need both items
      above; `crates/nvs-stdlib/src/script.rs:642` is the member all three go through.

## Backlog

- The two test hosts ignore the placement and have no core to assert on —
  `crates/nvs-runtime/src/host.rs:673`, `crates/nvs-runtime/src/stream.rs:591`; a seam test for it
  has somewhere to stand once `worker.rs` exists.
- `Core\Socket::upgrade` declines the same three options by name
  (`rule:concurrency/an-upgrades-options-are-spawn-scripts`); once `on:` is carried, whether the
  upgrade takes it is that rule's question, not this stage's.
- M5's `Task::map` speedup proof is stage 7's, over worker-placed children (goal § *Standing
  decisions*).
- The pack's traps missed the error-path gate over a new `Fault::` site; stage 6's `[context]
  playbook` now selects the two bullets that own it (`docs/agent/loop-goal.toml:227`).
