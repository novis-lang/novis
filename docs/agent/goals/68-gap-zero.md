---
milestone: post-parity
---
# Loop goal 68 — no gap is owed by anyone but a future milestone, and the index that held them is gone

The end of the gap program the user set on 2026-09-13. Every recorded gap is **closed, or tagged to a
future milestone (M9 and above) whose own plan file states the scope** — no item names a goal, no item
is `unowned`, no item is owed by a past milestone. **M0 through M8 — M4S and M4B included — are
complete**: `python tools/plan.py --past` says so for all eleven and their `Carried by` cells read `done`.
CI is green on `main`. `docs/agent/carried-gaps.md` is deleted, because an index of gaps with an owner
other than the plan has nothing left to index.

## Why here

**Last of the hand-written goals, directly in front of the dossier.** Everything that closes a gap runs
before it: goal `plan-truth` made the documents true, goal `gap-register` built the roster and the
past-milestone report this goal makes fatal, and goals `m4-refusals`, `m5-proofs`, `m4b-editor`,
`m7-server-surface`, `m8-db-queue`, `m8-stdlib-depth`, `unowned-closures` and `decided-closures` built
what the past milestones promised and what the user's decision sheet answered. Their acceptance lists are this goal's
floor, so their work is proven here by construction. **This goal builds nothing.** If its gate is red
when the run arrives, a closure goal went green without closing an item, and the item is named.

## Stage 0 — the catch-up

None.

## Stage 1 — the floor

Goal `decided-closures`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. It holds
every closure goal's checks. Never traded.

## Stage 2 — the gate becomes fatal

One file set: `tools/owners.py`, `crates/nvs-stdlib/tests/spec_registry_coverage.rs`, `tools/verify.py`.

1. **`unowned` stops being an owner kind.** `tools/owners.py:@classify` and `:@tag_of` accept a live goal
   slug or an M9+ milestone tag, nothing else; `crates/nvs-stdlib/tests/spec_registry_coverage.rs`'s
   `owner_problem` does the same for a ratchet key.
2. **The full gate is the default.** `owners.py --check` is fatal on an untagged item, a tag resolving to
   nothing, a goal owner, a past-milestone owner, and a retired owner. Every count on the summary is
   `label: N` (goal `gap-register`'s format), and the check reads five zeros.
3. **`verify.py` runs it** — `tools/verify.py`'s ordered run gains `owners.py --check`, so a gap written
   with a goal owner or none fails before it is committed.

## Stage 3 — the index is deleted, and nothing points at it

`docs/agent/carried-gaps.md` goes, and **every reader is re-pointed in the same slice** — a dangling
pointer is this goal's own failure one level up. At authoring time (`python tools/peek.py
"**/*:re:carried-gaps"` is the live list, and is re-run):

- `tools/owners.py` (its `unowned_paths` reads § *Unowned*; eleven mentions) and `tools/playbook.py`
  (`CARRIED_GAPS`, `owned_rows`, and the report section `:@report_expiry` prints)
- `tools/brief.py:471-483` (`HOMES` row and `OWNERS_HOME`), `tools/orient.py:1184`, `tools/goals.py:9`
- `crates/nvs-stdlib/tests/spec_registry_coverage.rs:78`, `:422`, `:475`, `:493` — the last uses the
  slug `carried-gaps` as its example of a live goal
- `crates/nvs-stdlib/tests/spec-members-outstanding.txt:4`, `spec-members-part-two-outstanding.txt:5`,
  `spec-classes-part-two-outstanding.txt:11` — their header comments
- `crates/nvs-stdlib/src/lib.rs:138`, `editors/vscode/src/regions.ts:30`
- `docs/agent/carried-refusals.md:10`, `docs/agent/session-prompt.md:122`,
  `docs/agent/loop-authoring.md:306` — § 8's destination for a gap found off the path becomes **the
  module doc that owns the code, with an owner tag**
- the playbook's bullets that name the file (`python tools/playbook.py --check` lists a path a bullet
  names that is gone)

**No floor check is traded.** The carried floor check *the durable gap list exists and every entry
names an owner* runs `python tools/chain.py --check`, which no longer reads this file
(`docs/agent/loop-goal.toml:4370-4375`), so it stays green when the file goes; its name is stale and
is left, because a floor is carried verbatim.

## Stage 4 — CI is green on `main`

M0's acceptance is "green on all three platforms in CI", and the closure goals added the extension,
database-matrix and ThreadSanitizer legs. `gh run list` must show the latest `ci.yml` run on `main`
completed with `success`. The user is fixing the GitHub billing block that stopped every run; if it is
still blocked when this stage is reached, that is a `BLOCKED` naming the billing block — the one hold this
goal expects — never a check rewritten to pass.

## Stage 5 — the past milestones are complete

`python tools/plan.py --past` reports all eleven past milestones complete, and `python tools/plan.py
--sync` has written `done` into each `Carried by` cell. The plan's status block's *Done* field is
rewritten whole to say M0–M8 are complete.

## Standing decisions

- **The user's rule, settled 2026-09-13**: every gap is closed or deferred to M9+; M0–M8 are complete.
  A milestone tag is right only when the item **cannot be built** until that milestone's work exists,
  never when it is merely large. `owners.py --deferrals` checks the mechanical half; the judgement is
  the closure goals', and it is re-checked here for any tag that arrived after them.
- **This goal builds nothing.** An open item found here is a `BLOCKED` naming the item and the closure
  goal that should have closed it — the run holds, the user schedules it. Inventing an owner, or
  deferring to a milestone to make the gate pass, is the one output this goal must not produce.
- **A gap found off the path goes in the module doc that owns the code**, tagged, and never into a new
  index. If no module owns it, it is not a gap — it is a feature request, and it is the user's.
- **What it spends**: nothing at run time. One tool invocation in `verify.py` over doc comments.
- **ADR slots**: none.
- **Not this goal**: any closure (goals `m4-refusals` through `unowned-closures`); the proofs every
  shipped feature owes (goal `dossier`).
