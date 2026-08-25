# Authoring a loop goal

Read this before rewriting `loop-goal.md` and `loop-goal.toml` for a new target. It carries the rules that
every goal so far has needed and that nobody should have to restate — what makes a goal drivable, what has
to be settled with the user *before* the run rather than during it, and the shape the stages take.

[coordinator.md](coordinator.md) owns how the loop is *driven*: the driver, the files, the acceptance-check
kinds, the per-iteration flow. This file owns how a goal is *written*. Neither restates the other.

## 1. Measure first — this is step zero, not a formality

    python tools/loop-stats.py

It reads `.loop/logs/` and re-derives every constant the loop's shape rests on: seconds per tool call, how
much of a session is fixed cost, how fast context grows, the model's actual context window, and the three
defensible answers for how many slices a session should take. **Run it before you write a line of the new
goal, and again after the run ends.**

This is not ceremony, and the history is the argument for it. Setting the group cap took three passes in
one afternoon: 4 from a single transcript, 2 when a second transcript was averaged in, and finally **1**
once the ceiling was fixed at 200k. Each answer was defensible from the evidence then in hand and each was
wrong. Every number here is a measurement of *this repository, on this machine, with this model*, and all
three change. A number you did not just measure is a number that is probably stale.

**The one fixed input is the ceiling: 200k, and it is a quality limit, not a capacity one.** The window is
1M and nothing compacts, but an agent starts missing what it has already read long before its window is
full, and a session that degrades produces work the acceptance test then rejects — which costs far more
than the session saved. `loop-stats.py` hard-codes it for that reason and reads the model's real window
only to catch the case where capacity binds first. Raise it only with evidence.

**Then reconsider the strategy, not just the number.** A shape follows from whichever budget binds, and
that has already flipped once here: with only the clock in view, over half of a session was fixed cost and
the answer was to group more slices per session; with the 200k ceiling in view, sessions were *already*
finishing over the line doing one slice, and the answer became to read less per session. Same repository,
same week, opposite instruction. So:

| If `loop-stats.py` now says | Then |
|---|---|
| Sessions finish **over** the ceiling | This is where the loop stands today. The cap is one slice and the lever is *reading less*: whole files only when small, regions otherwise, and nothing re-read that orientation already printed. Grouping is not available until sessions land under the line. |
| Sessions finish **well under** the ceiling | Grouping is back on. Take the cap the projection prints, preferring the knee over the fastest. |
| Fixed cost is a small share of a session | Grouping has stopped paying whatever the context says. Look at parallel lanes instead (coordinator.md's last section). |
| Sessions are compacting | The ceiling is far too high — compaction loses the standing instructions the run depends on. Drop it until it stops, and treat every result from that run as suspect. |
| Calls per message is above 1 | Batching finally happened, so the clock constants shifted but the context ones did not. Re-derive before trusting any earlier ratio; batching buys turns, never tokens. |
| `ctx_start` has crept up | The fixed cost of *existing* grew — AGENTS.md, the brief, the playbook. Every byte there is charged to every session before it does anything. |

Whatever you pick, **say in the commit which cap it is and why.** AGENTS.md § *Session workflow* step 2 is
the one place it lives.

## 2. A goal is a stop condition, or it is not a goal

The driver stops on an exit code, never on a session's opinion. So a goal must be expressible as commands
that exit 0 and output that matches exactly. **If it cannot be, do not run it unattended** — no reliable
stop condition means the run ends by exhausting `--max-sessions` with nobody watching.

Two failure modes worth naming, both of which have happened here:

- **A threshold is not a milestone.** The previous goal's acceptance list was reached with `Core` at 39 of
  ~205 spec members, because the list checked fixtures rather than coverage. If "complete" means every
  member, the check must be a test that *reads the spec* and fails naming what is missing. A count of
  conformance cases is a proxy; a test that enumerates the source of truth is not.
- **A green suite is not a run guard.** `cargo test` passes on a suite that never ran your new guard, so a
  named test must be checked for having *existed and run*. That is what `kind = "cargo-named"` is for, and
  it is why most of the tests a goal names do not exist when it is written: writing one is how an item
  finishes.

## 3. The two halves, and what belongs in each

| File | Holds | Never holds |
|---|---|---|
| `loop-goal.md` | The target, why it matters, the standing decisions, and the known gaps that sit on the path | The checks. Not one of them, not even summarised. |
| `loop-goal.toml` | Every check as data: fixtures, exact expected output, suites, named guard tests | Reasoning. A comment says what a check guards, not why the goal exists. |

The driver reads the TOML directly, so nothing in it can drift from what actually runs. `python
tools/loop.py --list` prints it as a summary; `--goal-only` runs it once without a session.

## 4. Pre-authorize every tradeoff, before the run

**Anything a session could reasonably stop and ask about will eventually halt the run on `BLOCKED`.** So
walk the path first and settle it with the user, then write each decision into `loop-goal.md`
§ *Standing decisions* as an instruction rather than a question. The current goal's section is the worked
example — dependency choices, representation choices, which language holes are in scope, which questions
are closed and must not be re-opened.

The house rule inside that section: **decide and record, never `BLOCKED` for a design call.** A session
settles it under AGENTS.md's priority ordering and records it in the home AGENTS.md already names. Reserve
`BLOCKED` for a decision that is expensive to reverse *and* has no safe default.

Two things every standing decision should carry: the safe fallback if the implementation forces the
opposite conclusion, and where the answer gets written down. A decision with no home gets re-derived.

## 5. The stages that have always worked

Stages run in order and short-circuit, so the ledger line names exactly how far the loop got. The shape
that keeps earning its place:

1. **Catch-up, if anything is owed** — decisions accepted *after* the milestone that owns their work was
   reported done. This runs before new breadth for one reason: every fixture written in the meantime is
   written against a rule that is about to change, and the rewrite compounds. The current goal's Stage 0
   had reached 48 files before it was paid off. Put it first or pay for it later, with interest.
2. **A non-regression floor** — the previous goal's whole acceptance list, unchanged, **never traded for
   anything above it.** This is what makes a long unattended run safe.
3. **The keystone**, if the goal has one — the single representation or mechanism everything else needs.
   If a fixture exists that cannot run until it lands, that fixture is the check.
4. **The breadth** — one fixture per domain, each reaching things no unit test proves reachable.
5. **The suites and the coverage gate** — see § 2 on why a count is not enough.
6. **The named guard tests**, then the leak sweep.

**Frozen output, unfrozen source.** A fixture's *expected output* may never be edited to make a check pass;
its *source* may be corrected freely, because whoever wrote it against the spec could not compile it. Say
in the commit message why a fixture was corrected. That distinction is what stops "make it pass" from
quietly becoming the goal.

## 6. Write the item list already grouped

The handoff names a **group** of slices sharing a file set, and a session takes as much of it as the
context ceiling allows — today that is one, sometimes two. Order the goal's items so those groups fall out
of the list, rather than leaving them to be invented at 2 a.m. by a session that has not read the other
items. The grouping is worth writing even while the cap is one: it is what lets a session pick the slice
whose files are cheapest to load, and it is ready the moment sessions come in under the line.

Group by **the files an item touches, not its topic.** Two items in the same function are one group; two
items about the same ADR in different crates usually are not. The shared file set is the entire mechanism:
it is what makes the second and third slice cost a fraction of the first. Where two adjacent items do not
share files, say so in the list — an item that gets its own session is a fine outcome, and pretending
otherwise costs a session a second orientation.

## 7. What never goes in a loop

- **Doc trimming.** Nothing measures doc size (AGENTS.md § *Length targets*), and
  [doc-cleanup.md](doc-cleanup.md)'s pass is fired by the user, never from inside a run.
- **Dependency sweeps.** Also the user's to fire ([dependency-update.md](dependency-update.md)).
- **Backlog work.** If a slice is not on the path to the acceptance list, it goes in the handoff's
  `## Backlog` and the session moves on.
- **Re-opening a standing decision.** That is what § 4 exists to prevent.
- **Anything needing judgement about whether the goal is met.** The machine outranks the claim: the driver
  runs the acceptance test itself and a session reporting `DONE` against a failing check stops the run.

## 8. When the run ends

Run `python tools/loop-stats.py` again. If the constants moved enough to change the cap or the strategy,
change them **and say so in the commit** — that is how the next goal starts from measurement rather than
from whatever this file happened to say. Fold anything durable the run taught you into the file that owns
it: a trap into [playbook.md](playbook.md), a shape into [conventions.md](conventions.md), a decision into
its ADR. `loop-goal.md` and `loop-goal.toml` are then rewritten from scratch for the next target, not
amended.
