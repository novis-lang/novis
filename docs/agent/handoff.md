# Handoff

## State

**Goal `class-scoped-types` is reached, and the floor check that held its DONE claim is green.** All
five stages are green, and the three gates a goal meets only at its end were re-run this session:
`python tools/verify.py --doc` resolves every link, and `python tools/owners.py --closes
class-scoped-types` and `python tools/playbook.py --closes class-scoped-types` each report the goal
owns nothing.

**The `abi-probe` floor failure was the guard, not the tree.** `a_cpu_bound_fan_out_across_four_worker_cores_is_near_linear_by_the_margin_this_test_names`
measured 0.65x inside the full `perf_guards` binary and 3.8x run alone: libtest had the other guards
on the cores it was fanning out onto. Every guard in that file now takes `serialised()` before it
measures, the binary is green in `6s`, and the fan-out reads the same figure in the binary as alone.
The playbook bullet under *Writing a test case* owns the trap.

**Next is goal `worker-placement`.** Its own `docs/agent/goals/62-worker-placement.handoff.md` is
what `goal-switch.py` installs; the group below is that file's, carried here so nothing is lost if
the switch is made by hand.

## Next group

**Stage 2: the seam a started core can install** — one file set:
`crates/nvs-runtime/src/script.rs`, `crates/nvs-host/src/placed.rs`, `crates/nvs-host/src/worker.rs`,
`crates/nvs-cli/src/main.rs`.

- [ ] **A resolver that can be published rather than borrowed** —
      `crates/nvs-runtime/src/script.rs:218`'s `install` takes a `&'static dyn Resolver` and `:241`'s
      `scoped` a borrow on the installing core's own stack, so neither reaches a thread `nvs-host`
      starts for itself. Add the form a placing core writes and a started core installs on its own
      thread, leaving `resolve`'s answer and `ResolveError` exactly as they are.
      `rule:security/isolate-shares-nothing` is what bounds what may be shared this way.
- [ ] **The started core installs it** — `crates/nvs-host/src/worker.rs:745`'s `destination` is where
      a core is chosen and a scheduler thread is started for the first placement; that start is where
      the published resolver goes in, beside the reactor the inbox poke reaches.
- [ ] **`crosses` stops asking which form the entry is** —
      `crates/nvs-host/src/placed.rs:107` is `entry.is_method() && ctx.class_table().is_some()`; the
      class-table half stays, because it is a fact about the context rather than about the entry.
      Rewrite that module's `# Known gaps` and `destination_for`'s second question
      (`crates/nvs-host/src/placed.rs:94`) as what then runs, and `crates/nvs-host/src/group.rs:89`'s
      restatement with them.

## Backlog

- `crates/nvs-cli/src/cache.rs`'s `a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names`
  is the same shape of ratio guard with no lock under it; its playbook bullet still says re-run alone.
- The pack's `modules` map describes no path under `benches/abi-probe/`, which hosts the floor's
  guards; the playbook trap naming that directory covered it, so it cost nothing here.
