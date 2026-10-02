# Authoring a loop goal

Read this before writing a goal: its prose at `docs/agent/goals/<slug>.md` and its record at
`data/goals/<slug>.json`. It carries the rules that
every goal so far has needed and that nobody should have to restate — what makes a goal drivable, what has
to be settled with the user *before* the run rather than during it, and the shape the stages take.

[coordinator.md](coordinator.md) owns how the loop is *driven*: the driver, the files, the acceptance-check
kinds, the per-iteration flow. This file owns how a goal is *written*. Neither restates the other.

**Place the goal on the chain first, then write its files by hand**: `bun nv chain --new <slug> --after
<goal>` puts the slug into `data/chain.json` and edits nothing else. A goal's place is its position in
that list, so no file is renamed when one is inserted. `bun nv chain --check` then names what the goal
still lacks — its prose, its record, its handoff record, a § *Why here* or § *Standing decisions*, a
`TODO` left in the prose — and what each of those wants is the sections below.

## 1. Measure first — this is step zero, not a formality

    bun nv loop-stats

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
than the session saved. `bun nv loop-stats` hard-codes it for that reason and reads the model's real window
only to catch the case where capacity binds first. Raise it only with evidence.

**Then reconsider the strategy, not just the number.** A shape follows from whichever budget binds, and
that has already flipped once here: with only the clock in view, over half of a session looked like fixed
cost and the answer was to group more slices per session; with the 200k ceiling in view, sessions were
*already* finishing over the line doing one slice, and the answer became to read less per session. Same
repository, same week, opposite instruction.

**That first reading was also wrong, which is worth keeping.** `tail` began at the first `nv verify` call,
and `--start` — the shape this repository recommends — fires it mid-work, so every edit after it was
counted as wrap-up. Corrected, the fixed cost is about 22%, not "over half", and the case for grouping was
always weaker than the number made it look. A measurement that flatters the strategy you already hold is
the one to re-derive first. So:

| If `bun nv loop-stats` now says | Then |
|---|---|
| Sessions finish **over** the ceiling | The lever is *reading less*: whole files only when small, regions otherwise, and nothing re-read that orientation already printed. Do **not** answer it with a slice count. A session that blew the ceiling inside its first slice never reached the group gate, so no count could have saved it — while a count does stop the sessions that finished with headroom to spare. The 120k gate is the one that binds, and it binds on context. |
| Sessions finish **well under** the ceiling | Grouping is paying. It needs no new number: the 120k gate lets a session keep taking slices exactly as long as it has room. Read the projection for the **ceiling**, not for a count of slices. |
| Fixed cost is a small share of a session | Grouping has stopped paying whatever the context says. Look at parallel lanes instead (coordinator.md's last section). |
| Sessions are compacting | The ceiling is far too high — compaction loses the standing instructions the run depends on. Drop it until it stops, and treat every result from that run as suspect. |
| Calls per message is above 1 | Batching finally happened, so the clock constants shifted but the context ones did not. Re-derive before trusting any earlier ratio; batching buys turns, never tokens. It has never happened by hand — 0 in 3,647 calls — which is why reading goes through `bun nv peek` instead. |
| `ctx_start` has crept up | The fixed cost of *existing* grew — AGENTS.md, the brief, the playbook. Every byte there is charged to every session before it does anything, and then re-billed on every turn of it. Over one run `playbook.md` grew 61% and dragged `ctx_start` up 5.3k with it; over the next, the pack went 59 KB to 118 KB at +907 B a session and the projection's cap fell to one slice on the strength of it alone. `--calibrate` prices a byte here; `bun nv session --wrap` reports the growth each session leaves behind; § 2 says what to do about it. |
| The same trap appears in the playbook twice | `bun nv playbook --show '<section>'` prints the section whole, so two wordings of one trap sit side by side. An append-mostly file cannot notice it already knows something: one trap had been written down six times, by six sessions, in six wordings, and each copy was charged to every session afterwards. |

**The projection opens where the *next* session will open**, not where the last ones did — the regressed
fixed floor plus the pack that is on disk right now. That matters when you have just changed what a
session reads: without it, a pass that halves the pack goes on producing the old cap until a whole further
run has been spent re-measuring what was just measured.

Whatever number you change there — the ceiling or the 120k gate — **say in the commit which one it is and
why.** AGENTS.md § *Session workflow* step 2 is the one place either lives, and it takes no slice count:
`bun nv loop-stats`'s three group-curve readings are a read of the curve, not a menu to install one of.

## 2. Scope the context — decide what a session may read, before deciding what it does

A goal is a **finite contained group of work**. It never needs the whole repository, and every byte a
session reads that the goal does not need is charged to the 200k ceiling exactly like a byte it did.
Before the run, write the `context` field of the goal's record, `data/goals/<slug>.json`. That field is
what `bun nv orient` slices the session's entire step 1 out of; without it there is no orientation but the
unscoped one, which is about 30k of context before a session has read a line of the code it came to change.

| Field | Selects | Get it wrong by |
|---|---|---|
| `modules` | globs under `crates/`; the map line for each | naming a crate when you meant a module, so the whole crate's map prints |
| `rules` | two entry forms, told apart by the `/` a rule id always has. **A rule id** — `"core-classes/schema-plan"` — prints that rule's fragment **whole**, which is the authoritative current text; name the two or three the item is written against. **A decision-record number** — `"0067"` — expands to the rules whose `because` names it: every one it **created** as `rule:` token plus title, a count of their guard paths, and the ids it **modified** — whole when there are six or fewer, only the header's count past that, never a sample. That form is the surrounding map, not the rule, and the titles are its cost — a record that created twenty rules prints twenty lines | naming only record numbers, so the pack carries titles and no rule text and the session pays a call to fetch one anyway; or listing every record the topic touches rather than the ones that *bind the work* — a foundational record sits in the `because` of sixty rules |
| `adrs` | `"NNNN"` for a record's *In short* block, `"NNNN §N"` for one section of `docs/decisions/NNNN.md` — frozen reasoning, for when the *why* is the question | naming a record whose rules are already in the rulebook: a record is history, and if the question is what is true now the answer is a rule id in `rules`. Naming a whole record is 7k of context where a section is 1k |
| `shapes` | headings of [conventions.md](conventions.md) the goal will write | listing all of them; a goal writing no `Core` member does not need that shape |
| `playbook` | a heading of [playbook.md](playbook.md), **or one bullet** — `"Tooling > a whole decision record"`, matched against the opening words of its bold lead-in. `bun nv chain --check` refuses a selector that opens several lead-ins unless it ends in `*`, which claims the whole family: `"Writing a test case > a -p*"`. The traps for reading a failing acceptance check are never named here: `bun nv orient` prints them whole when the driver reports a failing floor check, or one in a stage the handoff has passed, and as one line otherwise. Don't pick by hand: `bun nv orient --traps <path>...` ranks every bullet against the paths you give it, which are this goal's own `modules` | naming the section when the goal needs three of its bullets: sections grow forever, and this one is usually the pack's largest. Naming four whole sections cost 42 KB of a 78 KB pack until it was measured. `bun nv orient` narrows this list a second time, to the paths the session's own item names, so a selector that no item touches costs one line rather than a bullet |
| `plan` | status-block fields worth printing | more than `Open now` and `Blocking`, which is usually the answer |
| `milestones` | `"M4S"` for a whole milestone out of [docs/plan/](../plan/), `"M4S:lead"` or `"M4S:verify"` for one paragraph | naming the whole milestone when `:verify` was the question — M8 is 11k, its acceptance paragraph is under 1k |

### Write it per stage, not per goal

A goal is a finite contained group of work; **a stage of one is finite again**, and the manifest is the
place that distinction is worth money. Goal `lsp-server` runs twelve stages, and its stage 8 argues from ADR 0101's
redaction sections — which say nothing at all to the session writing its stage 4. Measured before this
existed: 13,120 of that goal's 17,746 B of sliced ADR text belonged to a stage either already landed or
not yet open, and its whole pack was 71,627 B.

So the record carries a `context` per **prose stage** — the `## Stage N` headings in the goal's own `.md`,
which are its `stages` list. The goal's own `context` is what a session needs whatever stage it is on;
a stage's `context` is what that stage needs on top of it:

```json
"context": {
  "rules": ["ide/an-lsp-answer-is-frozen-as-an-lspt-case", "0099"],
  "adrs": []
},
"stages": [
  {
    "number": 4,
    "title": "the requests",
    "context": {
      "rules": ["ide/an-open-document-is-its-own-entry-point", "ide/positions-have-one-home"],
      "adrs": ["0099 §1"]
    }
  }
]
```

- **`bun nv orient` applies the stage the handoff's `## Next group` names**, and `--stage N` prices one the
  run has not reached. A goal whose stages carry no `context` prints its base pack, which is what makes
  overlays safe to add to queued goals one at a time.
- **An overlay only ever adds.** Its entries are appended to the base's, deduplicated. The saving comes
  from keeping the *base* small — from moving an entry down into a stage, never from deleting one — and
  the worst a wrong stage number can do is print the base pack.
- **Narrowable: `rules`, `adrs`, `shapes`, `playbook`, `milestones`.** A stage's `context` takes the
  other two fields as well, and neither belongs there: not `modules`, because `bun nv goal context
  --add` writes to the goal's own list and a stage-local copy would silently stop receiving what a
  session added; not `plan`, whose default is two fields every session reads.
- **A stage is one number in both places.** A check's `stage` names one of the record's `stages`, and
  `bun nv chain --check` refuses a check whose stage the goal does not have.
- **`bun nv chain --check` audits every stage's entries**, not the one in flight, so a selector that resolves
  to nothing in stage 9 is caught while the goal is being written rather than by the session that opens
  stage 9 at 3am.

**Name rule ids.** This is the rule § 2's table has always stated and that no goal had ever once followed:
across the sixteen goals queued on 2026-09-07, `rules` held 4–10 entries each and **zero** rule ids
between them. A record number prints titles; the fragment is the rule. Two or three ids per stage, in that
stage's own table, is the shape — and it is *smaller* as well as more useful, because three rule bodies
run about 4 kB against the 15 kB of titles eight record numbers expand to.

Three rules make it work:

- **Every entry is a selector, never a copy.** `bun nv orient` slices the live file at session start, so a
  manifest cannot silently go stale the way a frozen context pack would. It can only go *wrong*, by naming
  something that no longer exists, and that prints as a loud warning.
- **A session widens `modules` itself.** The driver runs no context sweep, so a session that needed a
  module the manifest did not name adds it with `bun nv goal context --add <path>`, and the next
  session's pack prints it. Widening is the only thing that command can do, which is why it needs no
  supervision. Every other field is widened by editing the record: nothing on disk records that a
  session needed a rule section and did not get it.
- **An absent field selects nothing, not everything.** A goal that forgets to name its modules gets a short
  pack and a warning, rather than the whole map. Failing closed is what keeps the block honest.
- **`bun nv orient --audit` prints what the pack costs**, section by section. Look at it once,
  here, while writing the goal. It is a number, not a check — nothing exits non-zero over a size, and
  trimming prose against a tripwire is a cost this repository has already paid once
  ([doc-style.md](doc-style.md)).

**Write it from measurement, not from taste.** `bun nv loop-stats --attribute` charges the last
run's context to whatever fetched it, and each bucket argues for a specific fix: a large `adr` share means
whole records are being read where a rule or a `§` slice would do; a large `discovery` share means the checklist items
are missing their `file.rs:NN` anchors; a large `orientation` share means the manifest itself is too wide.

**Then leave it maintained by the sessions.** A session that needed something the pack did not print says
so in the handoff, naming the field; the next session that touches the goal adds the selector. A manifest
nobody may edit becomes a manifest everybody works around.

### What a new goal inherits, and what it owes

Most of the optimisation is **in the tools, and a new goal gets it for free**. Only two things are per-goal,
and only one of them is work.

| Carries over untouched | Because |
|---|---|
| `bun nv peek`, `session`, `verify`, `splice`, `plan`, `disk` | They are about how this repository is read and written, not about what any goal is doing. A new goal changes neither. |
| The rules you will otherwise break, and the session workflow, in `AGENTS.md` | Same. |
| The handoff contract, the playbook, `conventions.md` | Same. |
| The calibration in `tools/data/calibration.json` | Bytes per token is a property of the model and the pack's prose, not of the goal. Re-run `bun nv loop-stats --calibrate --write` when the *model* changes, not when the goal does. |

| Per-goal, and owed before the run | Cost |
|---|---|
| The record's `context`, **and a `context` on each of its `stages`** | The real work. It names the files, rules, record sections, shapes and traps *this* goal's sessions read, and nothing else knows them. The stage tables are where most of that lands: the base holds only what every stage needs. |
| A fresh `bun nv loop-stats` | One call. § 1 above: the constants are measurements, and a number you did not just measure is probably stale. |

So the answer to "do we have to re-do this every time" is **no for the tooling and yes for the manifest** —
and the manifest is not an optimisation you redo, it is the goal's own definition of what its sessions may
read. Write it once, let the sessions correct it, and the rest applies itself.

**One thing does drift on its own**: the pack's fixed floor. `playbook.md` is append-mostly by decision, so
every trap a session writes down is charged to every session after it. Three things hold that down — a
bullet's shape and weight ([conventions.md](conventions.md) § *A playbook bullet*; the wrap refuses a new
one past 700 bytes), the `[until:]` trailer the wrap retires bullets by, and `bun nv orient`'s cap on how many
print whole — but the manifest is still the lever a goal author holds: name **bullets** rather than
sections in the `context`'s `playbook` when a new goal only needs a few. `bun nv orient --audit`
prices both.

## 3. A goal is a stop condition, or it is not a goal

The driver stops on an exit code, never on a session's opinion. So a goal must be expressible as commands
that exit 0 and output that matches exactly. **If it cannot be, do not run it unattended** — a run is
uncapped by default, so no reliable stop condition means it does not end at all with nobody watching.

Three failure modes worth naming, all of which have happened here:

- **A threshold is not a milestone.** The previous goal's acceptance list was reached with `Core` at 39 of
  ~205 spec members, because the list checked fixtures rather than coverage. If "complete" means every
  member, the check must be a test that *reads the spec* and fails naming what is missing. A count of
  conformance cases is a proxy; a test that enumerates the source of truth is not.
- **A green suite is not a run guard.** `cargo test` passes on a suite that never ran your new guard, so a
  named test must be checked for having *existed and run*. That is what `"kind": "cargo-named"` is for, and
  it is why most of the tests a goal names do not exist when it is written: writing one is how an item
  finishes.
- **A chore is not a check.** Four goals ran `bun nv decisions --check`, which counts every
  decision not yet summarized — a pass the user fires, never a goal. A goal that opens an ADR is
  `missing` its own summary from the moment it writes the record, so all four were red on arrival and
  would each have held the run on a backlog no session of theirs could clear. A check has to be
  something the goal's own work turns green. `bun nv chain --check` does not look for a chore, so this
  is the author's to catch while the goal is written, not the session's when the chain reaches it.
- **A tag is not a build.** Goal `unowned-closures`'s gate was `unowned: 0` from the owners audit, and
  its stage 0 tagged every gap to the goal itself — which is what made the count zero, so the goal was
  reached with forty items tagged to it and none built. A gate over a register must ask what the
  goal's own tag cannot answer: `bun nv owners --closes <slug>` is red while any item names the goal.
  The driver's owner gate asks it, with `bun nv playbook --closes <slug>`, of every goal on the sweep
  that would reach it, whether or not the goal's list does, and a red gate holds the goal open; a
  session runs both itself before it writes `DONE`.

## 4. The two halves, and what belongs in each

| File | Holds | Never holds |
|---|---|---|
| `docs/agent/goals/<slug>.md` | The target, why it matters, the standing decisions, and the known gaps that sit on the path | The checks. Not one of them, not even summarised. |
| `data/goals/<slug>.json` | Every check as data: fixtures, exact expected output, suites, named guard tests — **and the `context` of § 2**, which is what a session may read | Reasoning. A check's `name` says what it guards, not why the goal exists. |

The driver reads the record directly, so nothing in it can drift from what actually runs. `bun nv loop
--list` prints the live goal's checks as a summary; `--goal-only` runs them once without a session; `bun
nv orient --audit` prints what its `context` costs a session, and `--goal <slug>` reads a goal that is
not live.

**Every session ends on one line that says whether the run needs you** — `nothing for you to do`, or
`YOUR HAND IS NEEDED` and the reason. `verdict()` in `tools/nv/driver/console.ts` writes it from what the driver
decided rather than from the session's own `CONTINUE`/`DONE`/`BLOCKED` line, because those two disagree
exactly when it matters: a session reports `CONTINUE` and the driver is stopping on a stall streak. On
that second line the run **holds** instead of ending, so answering it and pressing `p` is the whole of
what a `BLOCKED` costs — [coordinator.md](coordinator.md) § *Holding the tree* owns the rule.

The `context` lives with the checks rather than with the prose for the same reason the checks do: it is
read by a program, and a selector that names a section is either right or a loud warning. Prose about
*why* those are the files this goal touches belongs in the goal's `.md`.

## 5. Pre-authorize every tradeoff, before the run

**Anything a session could reasonably stop and ask about will eventually hold the run on `BLOCKED`.** So
walk the path first and settle it with the user, then write each decision into the goal's prose,
§ *Standing decisions*, as an instruction rather than a question. The current goal's section is the worked
example — dependency choices, representation choices, which language holes are in scope, which questions
are closed and must not be re-opened.

The house rule inside that section: **decide and record, never `BLOCKED` for a design call.** A session
settles it under AGENTS.md's priority ordering and records it in the home AGENTS.md already names. Reserve
`BLOCKED` for a decision that is expensive to reverse *and* has no safe default.

Two things every standing decision should carry: the safe fallback if the implementation forces the
opposite conclusion, and where the answer gets written down. A decision with no home gets re-derived.

**A goal that will open a record says so, and does not name the number.** A number is claimed by the
file that lands, one above the highest in `docs/decisions/` — so a number written into a goal near the
end of the chain is a number an earlier goal claims first, and the session that arrives finds it taken
and frozen. Say *one new record and no other number*; the goal `editor-install` form — naming the record because it
has already landed and this goal only implements it — is the other legitimate one.

`bun nv chain --check` notes a queued goal with no § *Standing decisions* at all, which is
the cheap half of this and the only half a tool can see. It cannot tell whether the section answers
the questions the stages actually reach — that is this section's judgement, and the goal whose design
record is not yet written is where it is hardest: implementing an accepted ADR asks a session to look
a decision up, while writing one asks it to make the decision at the moment it is least equipped to.

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
items about the same rule in different crates usually are not. The shared file set is the entire mechanism:
it is what makes the second and third slice cost a fraction of the first. Where two adjacent items do not
share files, say so in the list — an item that gets its own session is a fine outcome, and pretending
otherwise costs a session a second orientation.

**Every item carries its anchors.** The rule that specifies it (`rule:<topic>/<rule>`), and the `file.rs:NN` of the site it
changes. You are resolving them from context you already hold; a session without them spends ten `grep`s
rediscovering what you knew for free, and `bun nv loop-stats --attribute` charges that to the `discovery`
bucket where it shows up as a large share and an obvious fix. This is also how the `context` gets
written: the union of the anchors is the `modules` list.

## 8. What never goes in a loop

- **Doc trimming.** Nothing measures doc size ([doc-style.md](doc-style.md) § *Length targets*), and
  [doc-cleanup.md](doc-cleanup.md)'s pass is fired by the user, never from inside a run.
- **Dependency sweeps.** Also the user's to fire ([dependency-update.md](dependency-update.md)).
- **Backlog work.** If a slice is not on the path to the acceptance list, it goes in the handoff's
  `## Backlog` and the session moves on — **and if it will still be true after this goal ends it is a
  gap, so it is a record under `data/gaps/`** naming the module that owes it and the goal that owns it,
  where a goal switch cannot overwrite it. The handoff is state; a gap is not.
- **Re-opening a standing decision.** That is what § 5 exists to prevent.
- **Anything needing judgement about whether the goal is met.** The machine outranks the claim: the driver
  runs the acceptance test itself, a session reporting `DONE` against a failing check gets a retry
  session handed that check, and a retry whose `DONE` fails on the same check holds the run.

## 9. When the run ends

Run `bun nv loop-stats` again, and `bun nv loop-stats --attribute` beside it: the first says where the
sessions landed, the second says which *reads* put them there, and only the second tells you what to
change in the next goal's `context`. If the constants moved enough to change the cap or the
strategy, change them **and say so in the commit** — that is how the next goal starts from measurement rather than
from whatever this file happened to say. Fold anything durable the run taught you into the file that owns
it: a trap into [playbook.md](playbook.md), a shape into [conventions.md](conventions.md), a decision into
the rule's fragment under [docs/rules/](../rules/) with a new record for its reasoning. The next goal's
prose and record are written for its own target, never amended from this one's.
