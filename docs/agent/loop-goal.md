---
milestone: post-parity
---
# Loop goal 53 — one register reads every place a gap is written, and a milestone is an owner

`python tools/owners.py` answers "what is still owed, and by whom?" from every place the tree writes a
gap: the module docs, the refusal sites, the spec ratchets, the guard-name debt, the playbook's `[until:]`
bullets. It reads owed work under whichever heading a module doc used rather than only under
`# Known gaps`, it accepts a future milestone (M9 and above) as an owner and nothing earlier, and `python
tools/plan.py --past` says for each past milestone whether anything it promised is still open. What the
closure goals after this one must build is then derived rather than remembered.

## Why here

After goal `plan-truth`, which made the documents this goal reads true, and before the closure goals
(`m4-refusals` through `unowned-closures`), which each end on "nothing tagged to my milestone is left"
— a claim only this goal's roster can check. The audit of 2026-09-13 found the old register blind in
three places:
- `owners.py` read `# Known gaps` blocks only, and at least four modules record owed work under other
  headings (`crates/nvs-stdlib/src/test.rs:134`, `crates/nvs-cli/src/runner.rs:83`,
  `crates/nvs-render/src/lib.rs:12`, `crates/nvs-server/src/schedule.rs:31`).
- It accepted a tag naming any milestone the plan did not mark `done`, and `tools/plan.py` never writes
  `done` for a carried milestone (`tools/plan.py:@sync_schedule`), so M1, M6, M7 and M8 were valid
  deferral targets for 22 items.
- It could not see the six other registers at all, so "is anything open?" had six answers.

## Stage 0 — the catch-up

None beyond goal `plan-truth`'s.

## Stage 1 — the floor

Goal `plan-truth`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: one roster over every register

`tools/owners.py:@collect` and `:@report`, following `tools/holes.py:@main`'s derivation shape. **No
second tool**: a second index beside the first is the failure goals `carried-gaps` and `gap-owners` both
existed to end.

1. **`owners.py --registers`** prints one line per register it reads and the count of open items in
   each: `module docs`, `carried-refusals.md`, `outstanding keys` (the four
   `crates/nvs-stdlib/tests/*-outstanding.txt` ratchets and their `# <owner>` columns),
   `guard-name-debt.md`, `playbook until`, and — until goal `gap-zero` deletes it — `carried-gaps.md`.
   The last line is `N register(s)`. The roster and `--check` count every register's items.
2. **A milestone owner is a future milestone.** `tools/owners.py:@milestones` and `:@classify`: a tag
   names M9 or later — the plan's table lists M9, M10, M11, M12, M15, M16, M17 — or it is **owed by a past
   milestone**, which is a new report section and a count on the summary. **Every count the summary
   prints is written `label: N`** — `untagged: 0`, `past-milestone: 22`, `goal-owned: 7`,
   `unowned: 111`, `retired-owner: 6` — one per line, because an acceptance `want` is a substring match
   and `0 untagged` would also match `10 untagged`. Like a retired owner, it is reported and does not fail `--check` yet; goal `gap-zero`
   makes it fatal with `--past-is-an-error`. The same flag exists now, so the closure goals can run it
   over their own work.
3. **`owners.py --deferrals`** — every M9+ tag names a milestone whose own file under `docs/plan/` states
   the scope that covers the item, found by the module path or the item's keyword in that file. A
   miss prints the item and the milestone file; the last line is `every deferral names a future
   milestone whose plan states the scope` or a count of the misses.

## Stage 3 — the ratchet test learns the same two owner kinds

`crates/nvs-stdlib/tests/spec_registry_coverage.rs:419-441` (`owner_problem`) accepts a live goal or
`unowned` and its doc explains why a milestone could not be an owner for a key. Under the program's rule
a key owned by a future milestone is a deferral like any other, so `owner_problem` accepts an M9+ tag
from the plan's table, refuses an earlier milestone, and its doc is rewritten whole. Its test
`an_owner_that_is_not_a_live_chain_entry_fails` gains the milestone half.

## Stage 4 — owed work is written under one heading

Every module doc that records owed work under a heading other than `# Known gaps` has it moved into its
`# Known gaps` block as numbered items with an owner tag, and the old section rewritten to say what the
module does. Found at authoring time — `python tools/peek.py "crates/**/*.rs:re://! #+ .*(not yet|owe|still missing|not armed)"`
is the sweep, and it is re-run rather than trusted:

- `crates/nvs-stdlib/src/test.rs:134` — the in-process request's headers and body (goal
  `m7-server-surface`).
- `crates/nvs-cli/src/runner.rs:83` — § 2's test parallelism (owner decided by this stage; the runner is
  M8's `Core\Test`).
- `crates/nvs-render/src/lib.rs:12` — the HTML rendering and three producers (goal `m8-stdlib-depth`).
- `crates/nvs-server/src/schedule.rs:31` and `crates/nvs-cli/src/serve.rs:763` — a `fleet` entry is
  never armed under `nvs serve` (goal `m7-server-surface`).
- `crates/nvs-test/src/lib.rs:148`, `crates/nvs-db/src/lib.rs:100`, `crates/nvs-db/src/tds/mod.rs:54`,
  `crates/nvs-db/src/span.rs:30`, `crates/nvs-types/src/routes.rs:19`, `crates/nvs-types/src/commands.rs:21`,
  `crates/nvs-cli/src/script.rs:42`, `crates/nvs-server/src/route.rs:21` — read each; a section that
  records no owed work is left alone.
- `crates/nvs-stdlib/src/process.rs` has no gap block and `Core\Process::spawn` is owed; it gets one.

`owners.py` learns the sweep's heading pattern as a **warning**: owed-work wording under another
heading prints `sections outside Known gaps: N`, so the next one is caught when it is written.

## Stage 5 — a past milestone says whether it is complete

`python tools/plan.py --past` (`tools/plan.py:@main`, reading `owners.py --json`). For each milestone
before M9 in the plan's table — M0, M1, M2, M3, M4, M4S, M4B, M5, M6, M7, M8 — one line: the goals that
carry it and whether each has walked, and the count of items any register still tags to it. A milestone
is **complete** when every goal carrying it has walked and no item is tagged to it. The last line is
`N of 11 past milestone(s) complete`. `--sync` then writes `done` into a complete milestone's `Carried
by` cell, and `--check` accepts `done` only for a milestone `--past` calls complete. This is the
mechanical half of the user's rule; the judgement half is each closure goal's own acceptance list,
which the floor carries forward to goal `gap-zero`.

## Stage 6 — routing

`tools/brief.py:471-483` routes `gap`, `owner` and `register` to `docs/agent/carried-gaps.md`; it routes
to `tools/owners.py` instead, and `OWNERS_HOME`'s measured line reads the roster's counts. The index file
itself stays until goal `gap-zero` deletes it.

## Standing decisions

- **The user's rule, settled 2026-09-13**: M0–M8 are complete at the end of the program; only M9 and
  above is a valid deferral, and only when the item cannot be built without that milestone's work. This
  goal makes the first half of that mechanical; it does not close any gap itself.
- **Reported, not fatal, until goal `gap-zero`.** A past-milestone tag and an `unowned` tag stay legal here
  — the closure goals have not run — and both are counted on the last line so a green check never reads
  as a clean register. A gate that went red on work nobody has done yet would earn an exemption list.
- **`unowned` survives this goal.** Goal `unowned-closures` answers every one from the user's decision
  sheet, and goal `gap-zero` retires the kind.
- **A milestone's scope is its own file's.** `--deferrals` reads `docs/plan/mN.md`; it does not accept a
  scope sentence written anywhere else, and it never edits a plan file to make a tag pass.
- **What it spends**: nothing at run time — two tool modes and a test's match arm.
- **ADR slots**: none. The owner kinds are process, recorded in `tools/owners.py`'s module doc.
- **Not this goal**: closing any gap; deleting `carried-gaps.md` or re-pointing its readers (goal
  `gap-zero`); the decision sheet's answers (goal `unowned-closures`).
