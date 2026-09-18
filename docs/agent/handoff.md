# Handoff

## State

**Goal `bigint` is met — `Core\BigInt` is registered and every check in its acceptance list passes.**
The class, its 24 members, the six conformance cases plus the ordering case, `examples/bigint.nvs`'s
six frozen lines and the three named guard tests all landed in the previous run, under goal
`test-doubles`'s banner, because the chain advanced only after the work was already in. This session
built nothing: it confirmed the list and closed the goal.

`python tools/verify.py` is green whole — 4839 tests, 2083 conformance, 279 differential, 301 examples,
clippy clean. Stage 5's six gates each pass by hand: the compiler-facing ratchet holds no keys,
`docs/rules/core-api/tier-roster.md` no longer names big integers, and `rules.py --render --check`,
`owners.py --closes bigint`, `playbook.py --closes bigint` and `chain.py --check` are all clean.
`examples/bigint.nvs` prints the six lines `loop-goal.toml`'s stage-3 check freezes.

Nothing is blocked. Goal `gap-zero` is next; its own prose says it builds nothing, and its one
expected hold is CI.

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
