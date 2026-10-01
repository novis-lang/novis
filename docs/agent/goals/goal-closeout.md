---
milestone: post-parity
position: last
---
# Loop goal 183 — every old goal is closed, its decisions are kept, and the goals are deleted

Goal `ci-green` ends the program the chain was written for. This goal does three things, in order,
before the performance pass starts. First, it proves that every goal in front of it is really
finished. Second, it makes **a finished goal is deleted** the rule for every goal from here on.
Third, it deletes the old goals and everything that cited them.

When it is reached, the chain holds this goal and goal `performance-pass` and nothing else.
`docs/agent/goal-decisions.md` keeps every decision that only a goal's prose held, so nothing is lost
with the files. That file is for reading, not a rule. The test suites and `nv verify` are what protect
finished work from now on, and no carried floor does.

## Why here

**Behind goal `ci-green`, by the user's decision of 2026-10-01.** The audit asks whether the whole
program is finished. That question has an answer only once the last goal of the program is green, and
CI is that last goal.

**The audit and the deletion are one goal, not two.** Goal `ci-green`'s check is carried in this goal's
floor. It asks whether the code `HEAD` holds is what CI last ran. The first commit here that is not
bookkeeping turns it red, and only a push turns it green again. Two goals would mean a `BLOCKED` for a
push between the audit and the deletion. In one goal the check leaves with goal `ci-green`'s record
in Stage 6, and nothing waits on the user.

**The rule change is here, in the first new goal, by the user's decision of 2026-10-01.** Deletion is
the rule from this goal on, and never for a goal before it. Stage 5 lands it before Stage 6 uses it, so
the old goals are deleted by the same code that deletes every later goal. The driver then deletes this
goal itself when it switches to goal `performance-pass`.

**What this spends:** the carried floor. Today every walked goal's checks are run again for every goal
behind it. After this goal nothing is carried. The user's call is that the suites, the `.nvst` trees and
`nv verify` already cover everything a goal's checks proved. A check that proved something no suite
runs is found in Stage 3 and becomes a test before its goal is deleted.

## Stage 0 — the catch-up

None.

## Stage 1 — the floor

Goal `ci-green`'s whole carried list, run by the driver as it is.

**Its own check goes red at this goal's first commit that is not bookkeeping, and stays red until
Stage 6 deletes goal `ci-green`'s record.** That is expected. No session asks for a push because of it,
and no session edits that check to make it pass. Every other floor check is ordinary: red is work.

## Stage 2 — the registers

**Does:** Confirms that every register of owed work names only a milestone that is still ahead.

The registers were closed once already, by goal `gap-zero`. This stage proves they are still closed
after every goal walked since then. Every entry is either owned by a milestone at M9 or later, inside
the scope that milestone's file states, or it is a gap owned by `goal-closeout` and built in Stage 4.

- **The gap register.** `bun nv owners --check` and `bun nv owners --deferrals`. Every gap record under
  `data/gaps/` has a milestone owner whose `docs/plan/<id>.md` covers it.
- **The other registers.** `bun nv owners --registers` counts them: the refusal runs in
  `docs/agent/carried-refusals.md`, the keys in `crates/nvs-stdlib/tests/*-outstanding.txt`, the guard
  names in `docs/agent/guard-name-debt.md`, and the playbook bullets whose `until` names a state of the
  tree. Read each one. An entry that names owed work with no future milestone becomes a gap owned by
  `goal-closeout`.
- **The feature proofs.** `bun nv proofs --gate` owes nothing in the whole roster.
- **The plan.** `docs/implementation-plan.md`'s `Open now` and `Blocking` name nothing owed before M9.
  `bun nv plan --show M0:verify` through `M8:verify` are each met by the tree. A paragraph the tree
  does not meet becomes a gap owned by `goal-closeout`. A paragraph that promises less than the tree
  does is corrected: tested code that is ahead of a document wins, and the document is fixed.

## Stage 3 — every old goal is read

**Does:** Reads every goal in front of this one for unfinished work and for decisions that only its prose holds.

There are about 180 goals. This stage fans out to subagents: each one reads a batch of about fifteen
goals' prose, records and handoff records and reports back. It edits nothing. For each goal it answers
three questions.

1. **Is its work finished?** Compare what its stages promise, its handoff's `backlog`, and every
   `## Known gaps on the path` it names with the tree. Something promised and not built becomes a gap
   record owned by `goal-closeout`, with the `file:line` that owes it.
2. **Does a decision live only here?** A standing decision has a home when a rule fragment, a decision
   record, a module doc, the reference or a tested behaviour states it. A decision with no other home
   gets an entry in `docs/agent/goal-decisions.md`.
3. **Does a check prove something no suite runs?** A `cargo-named` test or an `nvs-suite` case already
   runs in `nv verify`. A `command` or a fixture check may not. One that proves a behaviour no suite
   covers becomes a test or a `.nvst` case in Stage 4.

`docs/agent/goal-decisions.md` opens by saying it is a record for reading and binds no goal, and that a
rule fragment is the only binding form. Each entry says what was decided, where in the tree it applies,
and the slug of the goal it came from, in backticks. A goal with nothing to keep is named in one list
at the end, so every slug appears once. The stage's check fails while any goal in front of this one is
missing from the file.

## Stage 4 — finished in place

**Does:** Builds every gap the audit found, so no gap names this goal.

Each gap owned by `goal-closeout` is built and its record deleted, or struck as a stated bound in its
module's prose, or given a milestone owner whose plan file covers it. That last answer is allowed only
when the work is genuinely that milestone's scope, never to clear the gate. Each missing test from
Stage 3's third question is written. The order is by file set, as always: gaps in one crate are one
group.

Something too large to finish here stops the run with `BLOCKED` naming it. "Too large" means it needs a
design record beyond the one Stage 5 opens, or it needs a decision only the user can make.

## Stage 5 — deletion becomes the rule

**Does:** Makes the goal switch delete the goal it leaves, and removes the floor machinery that nothing reads any more.

- **The record.** One new decision record, written first. It modifies
  `rule:tooling/the-chain-names-its-live-goal`: a walked goal is deleted at the switch, nothing is
  carried as a floor, and the suites are the safety net. It names this goal as the first goal the rule
  covers, and says that no goal before it was ever deleted by the rule. The fragment is rewritten whole,
  and [README.md](README.md) § *The chain contract* with it, rules 1 and 4 above all.
- **The switch.** `tools/nv/cmd/loop.ts:@advance` deletes the walked goal's prose, record and handoff
  record, removes its slug from `data/chain.json`'s `goals`, renders the goal plan, and commits all of
  it as the switch commit. The commit message keeps its form, ``docs(loop): the chain advances from `a`
  to `b` ``, and its body says the goal was deleted.
- **The floor.** `tools/nv/lib/chain.ts:@goalPlan` carries no floor, since no goal is ever in front of
  the live one. `walkedGoals`, the graduation of walked test checks in `tools/nv/cmd/chain.ts`, the
  retired-goal handling (`retired` in `tools/nv/cmd/owners.ts`, the empty-checks rule), and the floor
  stage in `tools/nv/driver/accept.ts` are deleted where nothing else reads them, with their tests.
  Simplicity is the reason, and the same commit says what each removal was.
- **The goal-end duty.** Before a session writes `DONE`, a decision that only its goal's prose holds is
  moved to its home: a rule, a record, a module doc. [session-prompt.md](../session-prompt.md) and
  [coordinator.md](../coordinator.md) say so, once, where each owns the goal end.
- **The citation check.** `bun nv chain --check` fails on prose that names a goal not on the chain, as
  it already fails on a goal named by its number. Decision records under `docs/decisions/` and
  `data/decisions/` are frozen history and exempt, and so is `docs/agent/goal-decisions.md`. A goal's
  own prose and handoff record are exempt too, because they are deleted with the goal. This file
  names goal `ci-green` after Stage 6 has deleted it, and that is correct.

After this stage the driver deletes this goal when it is reached. Its checks go with it, which is the
rule working as written.

## Stage 6 — the old goals are deleted

**Does:** Deletes every goal in front of this one and every citation of them.

- **The files.** For every slug in front of `goal-closeout`: `docs/agent/goals/<slug>.md`,
  `data/goals/<slug>.json` and its `.handoff.json`. `docs/agent/goals/dossier/` and any tool, test or
  fixture whose only job was one of those goals go too. `data/chain.json`'s `goals` starts at
  `goal-closeout`. It is edited by hand this one time, because `bun nv chain --remove` refuses a walked
  goal, and then `bun nv render` writes the goal plan.
- **The links first.** Do the deletion and the repair of every link it breaks in one slice, so `nv
  verify` is green at every commit. `bun nv links` names each broken link. The goal citations that are
  plain text come next, in groups by file set. `bun nv chain --check` names each of them.
- **The citers.** `docs/implementation-plan.md`, `docs/plan/*.md`, `docs/agent/*.md`, the playbook,
  module docs in `crates/`, `tools/nv/`, `tests/` and the examples. A sentence that cites a goal for its
  reasoning is rewritten to give the reason, or to cite the rule or record that holds it. A sentence
  that is only history is deleted, because `git log` keeps it.
- **The README.** [README.md](README.md)'s goal table holds `goal-closeout` and `performance-pass`, and
  its prose says nothing about generated goals or retired goals that is no longer true.

## Standing decisions

- **Finish in place, by the user's decision of 2026-10-01.** The audit does the work it finds. It
  stops on `BLOCKED` only for Stage 4's "too large".
- **No carried floor, by the user's decision of 2026-10-01.** The suites and `nv verify` protect
  finished work. Stage 3's third question is how a check that proved more than a suite becomes a test.
- **Deletion is the rule from this goal on, never before it.** The record says so. Goals walked before
  this one are deleted by Stage 6 as the one-time cleanup, not by the rule.
- **The driver changes here by the user's decision.** [coordinator.md](../coordinator.md) § *The
  loop's own machinery is a person's to change* is set aside for this goal alone. Its three conditions
  still hold for each commit: `bun nv orient` still produces a pack, `bun nv loop --list` still reads
  the list, and `nv verify` is green.
- **`docs/agent/goal-decisions.md` binds nobody.** It is the home of what it lists, and a later goal
  may decide differently without recording why. A decision a later goal must honour belongs in a rule.
- **Frozen records stay frozen.** A decision record that cites a deleted goal keeps the citation.
- **Tested code ahead of a document wins.** Code that does more than a goal or a plan file promised is
  correct, and the document is fixed. Code that lacks something decided is a gap.
- **No archive.** Nothing deleted is copied anywhere. `git log` is the history.
- **Subagents.** Stage 3 runs one subagent per batch of about fifteen goals. Each is read-only, stays
  under 250k of context, writes any scratch file under `.agent-tmp/` and deletes it before it reports.
  The session writes the gaps and `docs/agent/goal-decisions.md` from their reports.
- **ADR slots:** one new record, Stage 5's, and no other number.
