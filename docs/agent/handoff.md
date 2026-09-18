# Handoff

## State

**Goal `bigint` is met, and the one red left in its acceptance list was the machine rather than the
tree.** `Core\BigInt`, its 24 members, the six conformance cases plus the ordering case,
`examples/bigint.nvs`'s six frozen lines and the three named guard tests are all on disk; the
session before this one confirmed the list and closed the goal.

The floor's `abi-probe` check went red twice on
`a_cpu_bound_fan_out_across_four_worker_cores_is_near_linear_by_the_margin_this_test_names`, which
passes at 3.83x in 0.13 s on its own at this commit. The cause is the valgrind sweep that ends
0.14 s before it: the placed half of the ratio read 22.2 ms against 0.8 ms alone while the serial
half, which needs one core and no wake-ups, was unchanged at 3.4 ms. `tools/loop.py:3128`'s
`asked_again` re-asks a red `--release` check after `COST_SETTLE` (`tools/loop.py:1766`), so one red
is that shadow and a doubled one — `asked twice` in the failure line — is the tree.

Stage 5's gates are green by hand again: `verify.py --doc`, `owners.py --closes bigint`,
`playbook.py --closes bigint` and `chain.py --check`. Nothing is blocked. Goal `gap-zero` is next;
its own prose says it builds nothing, and its one expected hold is CI.

## Next group

**Goal `gap-zero`, stages 2 and 3: the fatal gate, then the index deleted** — one file set:
`tools/owners.py`, `tools/playbook.py`, `tools/brief.py`,
`crates/nvs-stdlib/tests/spec_registry_coverage.rs`. The switch installs that goal's own seed handoff
over this one (`tools/loop.py:3802`), so these three are its items with their anchors re-checked here.

- [ ] **Retire `unowned`** — `tools/owners.py:506`'s `classify`, `tools/owners.py:320`'s `tag_of`, and
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:505`'s `owner_problem`.
- [ ] **The full gate by default, run by `verify.py`** — `tools/owners.py:727`'s `run_check`, and
      `tools/verify.py`.
- [ ] **Delete `docs/agent/carried-gaps.md` and re-point every reader in the same slice** — the reader
      list is that goal's prose, `docs/agent/goals/70-gap-zero.md:1`, and `tools/check-links.py` is the
      proof.

## Backlog

- Stage 4 of `gap-zero`, CI green on `main` — `gh run list --branch main --workflow ci.yml`.
- Stage 5 of `gap-zero`, `python tools/plan.py --past` and `--sync` writing `done` — the plan's *Done*.
- Goal `bigint`'s `[context] modules` never named the four modules the ordering slice edited
  (`crates/nvs-stdlib/src/arr.rs`, `heap.rs`, `math.rs`, `ordering.rs`); moot as the goal retires,
  recorded because the driver flagged it and the next goal touching them pays for it again.
- The fifth artefact per feature, the `about.md` description prose, stays deferred to goal `dossier`.
