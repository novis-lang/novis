# Authoring a loop goal

Read this before rewriting `loop-goal.md` and `loop-goal.toml` for a new target. It carries the rules that
every goal so far has needed and that nobody should have to restate — what makes a goal drivable, what has
to be settled with the user *before* the run rather than during it, and the shape the stages take.

[coordinator.md](coordinator.md) owns how the loop is *driven*: the driver, the files, the acceptance-check
kinds, the per-iteration flow. This file owns how a goal is *written*. Neither restates the other.

**Start from the scaffold, not from a blank file or a copied neighbour**: `python tools/chain.py --new
<slug> --title "…"` writes the three files a chain entry needs with the boilerplate already carried
forward and every question this file answers left as a marked `TODO`, then splices the `[[goal]]` block
into `goals/chain.toml`. What each `TODO` wants is the sections below.

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
| Sessions finish **over** the ceiling | The lever is *reading less*: whole files only when small, regions otherwise, and nothing re-read that orientation already printed. Do **not** answer it with a slice count. A session that blew the ceiling inside its first slice never reached the group gate, so no count could have saved it — while a count does stop the sessions that finished with headroom to spare. The 120k gate is the one that binds, and it binds on context. |
| Sessions finish **well under** the ceiling | Grouping is paying. It needs no new number: the 120k gate lets a session keep taking slices exactly as long as it has room. Read the projection for the **ceiling**, not for a count of slices. |
| Fixed cost is a small share of a session | Grouping has stopped paying whatever the context says. Look at parallel lanes instead (coordinator.md's last section). |
| Sessions are compacting | The ceiling is far too high — compaction loses the standing instructions the run depends on. Drop it until it stops, and treat every result from that run as suspect. |
| Calls per message is above 1 | Batching finally happened, so the clock constants shifted but the context ones did not. Re-derive before trusting any earlier ratio; batching buys turns, never tokens. It has never happened by hand — 0 in 3,647 calls — which is why reading goes through `peek.py` instead. |
| `ctx_start` has crept up | The fixed cost of *existing* grew — AGENTS.md, the brief, the playbook. Every byte there is charged to every session before it does anything, and then re-billed on every turn of it. Over one run `playbook.md` grew 61% and dragged `ctx_start` up 5.3k with it; over the next, the pack went 59 KB to 118 KB at +907 B a session and the projection's cap fell to one slice on the strength of it alone. `--calibrate` prices a byte here; `session.py --wrap` reports the growth each session leaves behind; § 2 says what to do about it. |
| The same trap appears in the playbook twice | `python tools/playbook.py --dupes`. An append-mostly file cannot notice it already knows something: one trap had been written down six times, by six sessions, in six wordings, and each copy was charged to every session afterwards. |

**The projection opens where the *next* session will open**, not where the last ones did — the regressed
fixed floor plus the pack that is on disk right now. That matters when you have just changed what a
session reads: without it, a pass that halves the pack goes on producing the old cap until a whole further
run has been spent re-measuring what was just measured.

Whatever you pick, **say in the commit which cap it is and why.** AGENTS.md § *Session workflow* step 2 is
the one place it lives.

## 2. Scope the context — decide what a session may read, before deciding what it does

A goal is a **finite contained group of work**. It never needs the whole repository, and every byte a
session reads that the goal does not need is charged to the 200k ceiling exactly like a byte it did.
Before the run, write the `[context]` block in `loop-goal.toml`. That block is what `python
tools/orient.py` slices the session's entire step 1 out of; without it there is no orientation but the
unscoped one, which is about 30k of context before a session has read a line of the code it came to change.

| Field | Selects | Get it wrong by |
|---|---|---|
| `modules` | globs under `crates/`; the map line for each | naming a crate when you meant a module, so the whole crate's map prints |
| `rules` | ADR numbers; their one-sentence bullet from [ground-rules.md](../adr/ground-rules.md) | listing every ADR the topic touches rather than the ones that *bind the work* |
| `adrs` | `"NNNN"` for the *In short* block, `"NNNN §N"` for one section | naming a whole ADR — that is 7k of context where a section is 1k |
| `shapes` | headings of [conventions.md](conventions.md) the goal will write | listing all of them; a goal writing no `Core` member does not need that shape |
| `playbook` | a heading of [playbook.md](playbook.md), **or one bullet** — `"Tooling > A whole ADR"`. Don't pick by hand: `python tools/playbook.py --goal` ranks every bullet against this goal's own `modules` and prints the list as TOML | naming the section when the goal needs three of its bullets: sections grow forever, and this one is usually the pack's largest. Naming four whole sections cost 42 KB of a 78 KB pack until it was measured. `orient.py` narrows this list a second time, to the paths the session's own item names, so a selector that no item touches costs one line rather than a bullet |
| `plan` | status-block fields worth printing | more than `Open now` and `Blocking`, which is usually the answer |
| `milestones` | `"M4S"` for a whole milestone out of [docs/plan/](../plan/), `"M4S:lead"` or `"M4S:verify"` for one paragraph | naming the whole milestone when `:verify` was the question — M8 is 11k, its acceptance paragraph is under 1k |

Three rules make it work:

- **Every entry is a selector, never a copy.** `orient.py` slices the live file at session start, so a
  manifest cannot silently go stale the way a frozen context pack would. It can only go *wrong*, by naming
  something that no longer exists, and that prints as a loud warning.
- **An absent field selects nothing, not everything.** A goal that forgets to name its modules gets a short
  pack and a warning, rather than the whole map. Failing closed is what keeps the block honest.
- **`python tools/orient.py --audit` prints what the pack costs**, section by section. Look at it once,
  here, while writing the goal. It is a number, not a check — nothing exits non-zero over a size, and
  trimming prose against a tripwire is a cost this repository has already paid once
  ([doc-style.md](doc-style.md)).

**Write it from measurement, not from taste.** `python tools/loop-stats.py --attribute` charges the last
run's context to whatever fetched it, and each bucket argues for a specific fix: a large `adr` share means
whole ADRs are being read where a `§` slice would do; a large `discovery` share means the checklist items
are missing their `file.rs:NN` anchors; a large `orientation` share means the manifest itself is too wide.

**Then leave it maintained by the sessions.** A session that needed something the pack did not print says
so in the handoff, naming the field; the next session that touches the goal adds the selector. A manifest
nobody may edit becomes a manifest everybody works around.

### What a new goal inherits, and what it owes

Most of the optimisation is **in the tools, and a new goal gets it for free**. Only two things are per-goal,
and only one of them is work.

| Carries over untouched | Because |
|---|---|
| `peek.py`, `session.py`, `verify.py`, `splice.py`, `plan.py`, `disk.py` | They are about how this repository is read and written, not about what any goal is doing. A new goal changes neither. |
| The five rules and the session workflow in `AGENTS.md` | Same. |
| The handoff contract, the playbook, `conventions.md` | Same. |
| The calibration in `tools/data/calibration.json` | Bytes per token is a property of the model and the pack's prose, not of the goal. Re-run `--calibrate --write` when the *model* changes, not when the goal does. |

| Per-goal, and owed before the run | Cost |
|---|---|
| The `[context]` manifest | The real work. It names the files, ADR sections, shapes and traps *this* goal's sessions read, and nothing else knows them. |
| A fresh `python tools/loop-stats.py` | One call. § 1 above: the constants are measurements, and a number you did not just measure is probably stale. |

So the answer to "do we have to re-do this every time" is **no for the tooling and yes for the manifest** —
and the manifest is not an optimisation you redo, it is the goal's own definition of what its sessions may
read. Write it once, let the sessions correct it, and the rest applies itself.

**One thing does drift on its own**: the pack's fixed floor. `playbook.md` is append-mostly by decision, so
every trap a session writes down is charged to every session after it. Over one 39-session run it grew 61%
and pulled `ctx_start` up by 5.3k tokens — which, re-billed on ~98 turns, is about half a million tokens a
session. The fix is not to trim the playbook; it is to name **bullets** rather than sections in `[context]
playbook` when a new goal only needs a few. `python tools/orient.py --audit` prices both.

## 3. A goal is a stop condition, or it is not a goal

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

## 4. The two halves, and what belongs in each

| File | Holds | Never holds |
|---|---|---|
| `loop-goal.md` | The target, why it matters, the standing decisions, and the known gaps that sit on the path | The checks. Not one of them, not even summarised. |
| `loop-goal.toml` | Every check as data: fixtures, exact expected output, suites, named guard tests — **and the `[context]` block of § 2**, which is what a session may read | Reasoning. A comment says what a check guards, not why the goal exists. |

The driver reads the TOML directly, so nothing in it can drift from what actually runs. `python
tools/loop.py --list` prints it as a summary; `--goal-only` runs it once without a session; `python
tools/orient.py --audit` prints what its `[context]` block costs a session.

The `[context]` block lives with the checks rather than with the prose for the same reason the checks do:
it is read by a program, and a selector that names a section is either right or a loud warning. Prose about
*why* those are the files this goal touches belongs in `loop-goal.md`.

## 5. Pre-authorize every tradeoff, before the run

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

## 6. The stages that have always worked

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
5. **The suites and the coverage gate** — see § 3 on why a count is not enough.
6. **The named guard tests**, then the leak sweep.

**Frozen output, unfrozen source.** A fixture's *expected output* may never be edited to make a check pass;
its *source* may be corrected freely, because whoever wrote it against the spec could not compile it. Say
in the commit message why a fixture was corrected. That distinction is what stops "make it pass" from
quietly becoming the goal.

## 7. Write the item list already grouped

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

**Every item carries its anchors.** The ADR section that specifies it, and the `file.rs:NN` of the site it
changes. You are resolving them from context you already hold; a session without them spends ten `grep`s
rediscovering what you knew for free, and `loop-stats.py --attribute` charges that to the `discovery`
bucket where it shows up as a large share and an obvious fix. This is also how the `[context]` block gets
written: the union of the anchors is the `modules` list.

## 8. What never goes in a loop

- **Doc trimming.** Nothing measures doc size ([doc-style.md](doc-style.md) § *Length targets*), and
  [doc-cleanup.md](doc-cleanup.md)'s pass is fired by the user, never from inside a run.
- **Dependency sweeps.** Also the user's to fire ([dependency-update.md](dependency-update.md)).
- **Backlog work.** If a slice is not on the path to the acceptance list, it goes in the handoff's
  `## Backlog` and the session moves on.
- **Re-opening a standing decision.** That is what § 5 exists to prevent.
- **Anything needing judgement about whether the goal is met.** The machine outranks the claim: the driver
  runs the acceptance test itself and a session reporting `DONE` against a failing check stops the run.

## 9. When the run ends

Run `python tools/loop-stats.py` again, and `python tools/loop-stats.py --attribute` beside it: the first
says where the sessions landed, the second says which *reads* put them there, and only the second tells you
what to change in the next goal's `[context]` block. If the constants moved enough to change the cap or the
strategy, change them **and say so in the commit** — that is how the next goal starts from measurement rather than
from whatever this file happened to say. Fold anything durable the run taught you into the file that owns
it: a trap into [playbook.md](playbook.md), a shape into [conventions.md](conventions.md), a decision into
its ADR. `loop-goal.md` and `loop-goal.toml` are then rewritten from scratch for the next target, not
amended.
