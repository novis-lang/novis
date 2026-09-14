# Handoff

## State

**Goal `gap-register` is met — every stage's checks pass.** Stage 5 landed this session: `python
tools/plan.py --past` prints one line per pre-M9 milestone and ends `4 of 11 past milestone(s)
complete`, `--sync` writes `done` into a complete one's `Carried by` cell, and `--check` refuses that
cell anywhere else.

- Completeness has one home, `tools/plan.py:@past_state` — every goal carrying the milestone has
  walked, and no register still tags an item to it. `--past`, `--sync` and `--check` all read it, so
  the report, the cell and the gate cannot disagree.
- `python tools/owners.py --json` now carries each register's owner tally and `first_future_milestone`,
  so the count is taken across all six registers rather than the module docs alone: M7's four are three
  `# Known gaps` items and one `carried-gaps.md` § *Owned* row.
- M4S's cell is `done` — `core-depth` walked and nothing tags M4S. `docs/implementation-plan.md:73` is
  where the column's vocabulary says so.
- The floor's `the five-driver matrix`, red at the last acceptance sweep, was a cold-boot wait and not a
  regression: `python tools/db-matrix.py --all` answers `5/5 drivers ok` on this tree.

Nothing is blocked. Run `cargo test` with nothing else loading the machine — `nvs-cli`'s
`a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile` asserts a timing bound and
failed once here beside four booting database containers, green on its own.

## Next group

**The goal is met, so the next group is the next chain entry's** — goal `unowned-closures`, whose own
opening handoff is already on disk and replaces this file at the switch. Nothing in `gap-register` is
open; the one item below is the re-read that switch wants, not a slice of this goal.

- [ ] **Take goal `unowned-closures`'s opening group from its own handoff** — the 48 `unowned` items
      `python tools/owners.py --unowned` lists, each answered from the user's decision sheet:
      `docs/agent/goals/60-unowned-closures.handoff.md:13`, against `docs/agent/carried-gaps.md:59`.

## Backlog

- The 21 items tagged to a milestone behind the program — `python tools/owners.py --check
  --past-is-an-error` names them; each closure goal's acceptance list owns the judgement half.
- `unowned` retires as an owner kind in goal `gap-zero`, which also deletes `carried-gaps.md`.
- `docs/agent/carried-gaps.md` § *Unowned* carries a reason per unowned module — goal
  `unowned-closures` is what empties it.
