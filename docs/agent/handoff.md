# Handoff

## State

**Goal `gap-register` — one register reads every place a gap is written, and a milestone is an
owner. Stage 2 is complete; stage 1's floor is green.** `tools/owners.py` now has all three modes
the goal's stage 2 names, and its module doc is their home.

- `--registers` prints the six registers and what each holds now: module docs 154,
  carried-refusals.md 1, outstanding keys 5, guard-name-debt.md 0, playbook until 274,
  carried-gaps.md 79. It takes only the *count* from the other five — each already has a gate over
  its own discipline — and asks `tools/playbook.py` what an entry is rather than copying the
  predicate (`tools/owners.py:@entries`).
- A milestone tag must name M9 or later (`tools/owners.py:@is_past`). 21 items name an earlier one;
  they are a report section and a `past-milestone: 21` count, refused only by `--past-is-an-error`,
  per the goal's § *Standing decisions*. Every count the check prints is now a `label: N` line.
- `--deferrals` checks each M9+ tag against `docs/plan/mN.md` by path, by a backticked name, then by
  a word of the item's own text (`tools/owners.py:@covers`). All 10 pass.

Nothing is blocked, and no gap was closed — this goal builds the register, it does not empty it.

## Next group

**Stage 3: the ratchet test learns the same two owner kinds** — one file set:
`crates/nvs-stdlib/tests/spec_registry_coverage.rs`.

- [ ] **`owner_problem` accepts a future milestone and refuses a past one** —
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:433`. An M9-or-later tag from the plan's
      table (`docs/implementation-plan.md`'s rows, which nothing in this test file reads yet) is a
      deferral like any other; an earlier one is not. The rule is the goal's stage 3 and
      `tools/owners.py`'s module doc § *A milestone tag*; `FIRST_FUTURE_MILESTONE` is the number's
      one home in Python and the test needs its own.
- [ ] **The doc at `crates/nvs-stdlib/tests/spec_registry_coverage.rs:419` is rewritten whole** —
      it currently argues that a milestone can never own a key, which stage 3 reverses. Rewrite it
      from what the function does then, per `AGENTS.md` rule 7; do not leave the old sentence
      beside the new one.
- [ ] **Both tests** — `an_owner_that_is_not_a_live_chain_entry_fails`
      (`crates/nvs-stdlib/tests/spec_registry_coverage.rs:491`) gains the milestone half, and
      `a_future_milestone_owns_a_key_and_a_past_one_does_not` is written beside it. Those two names
      are the acceptance check's, `docs/agent/loop-goal.toml:10102`.

## Backlog

- Stage 4: the module docs that record owed work outside `# Known gaps`, and `owners.py` learning
  the sweep as a warning printing `sections outside Known gaps: N` — goal file § *Stage 4*.
- Stage 5: `python tools/plan.py --past`, reading `owners.py --json` (which now carries a
  `registers` object and a `past` kind) — goal file § *Stage 5*.
- Stage 6: `tools/brief.py:471-505` routes `gap`/`owner`/`register` to `carried-gaps.md`; it routes
  to `tools/owners.py`, and `ownership_line()` reads the roster's counts.
- `--deferrals`' keyword match is a heuristic with a prose stoplist (`tools/owners.py:@covers`,
  `PROSE`); if it ever scopes an item by a word that says nothing, tighten it there.
