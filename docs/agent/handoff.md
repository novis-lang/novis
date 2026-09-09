# Handoff

## State

**Goal 38 is met: all four stages are green.** Stage 3's check names
`every_encoder_that_reaches_an_object_graph_refuses_a_cycle_rather_than_recursing`, and it is now in
`crates/nvs-stdlib/src/json.rs`'s test module over that module's own object fixture: one cyclic
`Holder` is handed to `Core\Json::encode`, `Core\Csv::format`, `Core\Uri::buildQuery` and
`Core\Encoding::toHex` through `nvs_runtime::call`, each refuses, and the counting assertion is that
exactly one of them — the only walk that descends into an object — answers by identity. `Core\Serialize`
is not asked: goal prose stage 3 puts it under `rule:classes/graph-copy`.

**Stage 4 landed with it.** `rule:classes/an-encoder-ends-a-cycle-by-identity` is `shipped`, its
`guardedBy` names the conformance case and `crates/nvs-stdlib/src/json.rs`, and
`docs/rules/classes.md` and `docs/ground-rules.md` are re-rendered. `rules.py --check`,
`rules.py --render --check` and `decisions.py --gate` all pass.

`decisions.py --check` still reports 0164 unsummarized — that is the user-fired chore
`docs/agent/decisions-summary.md` owns, and no goal gates on it.

## Next group

**Nothing is open in goal 38** — one file set, and only if the driver's sweep disagrees with the
line above:

- [ ] **Read the check the sweep names before writing anything.** Stage 3's block is
      `docs/agent/loop-goal.toml:6365` and the test that answers it is
      `crates/nvs-stdlib/src/json.rs:2790`; stage 4's three `command` checks are the blocks after it
      and all three pass on this tree. A check still red here is a fact about that block rather than
      about the tree. `rule:classes/an-encoder-ends-a-cycle-by-identity`.

## Backlog

- `python tools/decisions.py --work` — 0164 and sixteen other records are unsummarized
  (`docs/agent/decisions-summary.md`).
- The encode walk's frame budget is not the depth cap — `crates/nvs-stdlib/src/json.rs`'s own gap 7,
  and the playbook holds the trap.
- `crates/nvs-stdlib/src/serialize.rs` carries no audit sentence on purpose; goal prose stage 3 says
  why.
