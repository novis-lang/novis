# Authoring a loop goal

Read this before writing a goal: one file, `docs/goals/<slug>.md`, in looper's goal format (looper's own
`docs/goal-format.md` is that format's home). It carries the rules that every goal so far has needed and
that nobody should have to restate — what makes a goal drivable, what has to be settled with the user
*before* the run rather than during it, and the shape the stages take.

[coordinator.md](coordinator.md) owns how the loop is *driven*. This file owns how a goal is *written*.
Neither restates the other.

**Place the goal on the chain first, then write its prose by hand**: `looper goal new <slug> --title
<title> --summary <reached when>` writes a file with every section and one open stage, last on the
chain, and `looper goal move <slug> <position>` puts it where it belongs. A goal's place is its position
in the `chain:` list of `docs/goals/README.md`, so no file is renamed when one is inserted. `looper lint`
then names what the file still lacks, and what each section wants is below.

## 1. Measure first — this is step zero, not a formality

Read the last run's sessions before you write a line of the new goal, and again after the run ends:
`looper log <session>` prints one session's events, and a handful of them is the evidence for how much a
session reads before its first edit, how fast its context grows and where it ended.

Every number the loop's shape rests on is a measurement of *this repository, on this machine, with this
model*, and all three change. A number you did not just measure is a number that is probably stale.

**The one fixed input is the ceiling, and it is a quality limit, not a capacity one.** An agent starts
missing what it has already read long before its window is full, and a session that degrades produces
work the acceptance test then rejects — which costs far more than the session saved. AGENTS.md
§ *Session workflow* step 2 is the ceiling's only home, and `looper.toml`'s `[budget]` repeats it for the
driver. Raise it only with evidence.

**Then reconsider the strategy, not just the number.** A shape follows from whichever budget binds: with
the clock binding, grouping more slices per session is the answer; with the ceiling binding, it is
reading less per session. A measurement that flatters the strategy you already hold is the one to
re-derive first. So:

| If the sessions show | Then |
|---|---|
| Sessions finish **over** the ceiling | The lever is *reading less*: whole files only when small, regions otherwise, and nothing re-read that orientation already printed. Do **not** answer it with a slice count. A session that blew the ceiling inside its first slice never reached the group gate, so no count could have saved it. The 120k gate is the one that binds, and it binds on context. |
| Sessions finish **well under** the ceiling | Grouping is paying. It needs no new number: the 120k gate lets a session keep taking slices exactly as long as it has room. |
| Fixed cost is a small share of a session | Grouping has stopped paying whatever the context says. Look at parallel lanes instead (coordinator.md's last section). |
| Sessions are compacting | The ceiling is far too high — compaction loses the standing instructions the run depends on. Drop it until it stops, and treat every result from that run as suspect. |
| The context at a session's start has crept up | The fixed cost of *existing* grew — AGENTS.md, the pack, the playbook. Every byte there is charged to every session before it does anything, and then re-billed on every turn of it. § 2 says what to do about it. |
| The same trap appears in the playbook twice | An append-mostly file cannot notice it already knows something, and each copy is charged to every session afterwards. Delete the copy. |

Whatever number you change — the ceiling or the 120k gate — **say in the commit which one it is and
why.** AGENTS.md § *The context budget* is the one place either lives, and it takes no slice count.

## 2. Scope the context — decide what a session may read, before deciding what it does

A goal is a **finite contained group of work**. It never needs the whole repository, and every byte a
session reads that the goal does not need is charged to the ceiling exactly like a byte it did. Before
the run, write the goal's `context` list: the files whose head `looper orient` prints into the pack.
Without it a session's first calls are spent finding the code it came to change.

**Write it per stage, not per goal.** A stage of a goal is finite again, and the manifest is the place
that distinction is worth money: stage 8 of a twelve-stage goal can argue from files that say nothing at
all to the session writing stage 4. So the goal's own `context` is what a session needs whatever stage
it is on, and a stage's `context` is what that stage needs on top of it:

```yaml
context: [crates/nvs-lsp/src/lib.rs]
stages:
  - id: 4
    title: the requests
    status: open
    context: [crates/nvs-lsp/src/requests.rs, docs/rules/ide/positions-have-one-home.md]
```

- **An overlay only ever adds.** The saving comes from keeping the *base* small — from moving an entry
  down into a stage, never from deleting one.
- **Name the rule fragment.** A rule's fragment under `docs/rules/` is what is true now, with its
  reasons and its Declined list. Two or three fragments per stage is the shape.
- **A goal's other data goes in `extra`.** `files` and `env` are Novis's own fields, which
  looper never reads inside, and `tools/nv/lib/chain.ts` is what reads them.

Three rules make it work:

- **Every entry is a path, never a copy.** `looper orient` reads the live file at session start, so a
  manifest cannot silently go stale. It can only go *wrong*, by naming something that no longer exists,
  and that prints as one line.
- **A session widens `context` itself.** A session that needed a file the manifest did not name adds it
  with `looper goal context add <slug> <path>`, or `--stage <id>` for one stage, and the next session's
  pack prints it.
- **An absent entry selects nothing, not everything.** A goal that forgets to name a file gets a short
  pack, rather than the whole map. Failing closed is what keeps the list honest.

**Then leave it maintained by the sessions.** A session that needed something the pack did not print says
so in the handoff; the next session that touches the goal adds it. A manifest nobody may edit becomes a
manifest everybody works around.

### What a new goal inherits, and what it owes

Most of the optimisation is **in the tools, and a new goal gets it for free**: `bun nv peek`, `verify`,
`splice` and `disk`, the rules and the session workflow in `AGENTS.md`, the handoff contract,
the playbook and `conventions.md`. They are about how this repository is read and written, not about
what any goal is doing.

What is per goal and owed before the run is the `context`, **and a `context` on each of its stages** —
the real work, because nothing else knows which files *this* goal's sessions read. Write it once, let
the sessions correct it, and the rest applies itself.

**One thing does drift on its own**: the pack's fixed floor. The playbook is append-mostly by decision,
so every trap a session writes down is charged to every session after it. A bullet's shape and weight
([conventions.md](conventions.md) § *A playbook bullet*; `looper wrap` refuses one past 700 bytes) and the
`until` trailer `looper playbook retire` deletes it by are what hold that down.

## 3. A goal is a stop condition, or it is not a goal

looper stops on an exit code, never on a session's opinion. So a goal must be expressible as commands
that exit 0 and output that matches exactly. **If it cannot be, do not run it unattended** — no reliable
stop condition means the run does not end with nobody watching.

Failure modes worth naming:

- **A threshold is not the goal.** An acceptance list that checks fixtures rather than coverage is
  reached long before the work is. If "complete" means every member, the check must be a test that
  *reads the source of truth* and fails naming what is missing. A count of
  conformance cases is a proxy; a test that enumerates the source of truth is not.
- **A green suite is not a run guard.** `cargo test` passes on a suite that never ran your new guard, so a
  named test must be checked for having *existed and run*. That is what a check's `tests` list is for,
  and it is why most of the tests a goal names do not exist when it is written: writing one is how an
  item finishes. A name that matches nothing fails its check and holds the goal open, for one of three
  causes. The work landed under another name: rewrite the name in the check. The work was pinned by a
  `.nvst` case instead: move the check to the suite that owns it. The test is unwritten: write it, and
  never rename it to something green, which hides an open hole. A session that lands a guard test
  reconciles its name in the goal's check in the same slice.
- **A chore is not a check.** A check that counts a backlog across the whole tree — every doc not yet
  swept, every item not yet summarized — is a pass the user fires, never a goal. It is red on arrival
  and holds the run on work no session of the goal can clear. A check has to be
  something the goal's own work turns green. `looper lint` does not look for a chore, so this is the
  author's to catch while the goal is written.
- **A tag is not a build.** A gate of `unowned: 0` from the owners audit turns green the moment a
  stage tags every gap to the goal itself, with none of them built. A gate over a register must ask what the
  goal's own tag cannot answer: `bun nv owners --closes <slug>` is red while any item names the goal,
  and a session runs it itself before it writes `DONE`.

## 4. The two halves, and what belongs in each

A goal is one file, and its two halves are kept apart inside it:

| Half | Holds | Never holds |
|---|---|---|
| The front matter | Every check as data — the command, the exit, the exact expected output, the named tests — **and the `context` of § 2**, which is what a session may read | Reasoning. A check's `name` says what it guards, not why the goal exists. |
| The prose sections | The target, why it matters, the standing decisions, the stages and their items, and the handoff | The checks. Not one of them, not even summarised. |

looper reads the front matter directly, so nothing in it can drift from what actually runs. `looper goal
show` prints a goal's stages and checks, `looper check` runs the live goal's acceptance once without a
session, and `looper orient` prints what its `context` gives a session.

The `context` lives with the checks rather than with the prose for the same reason the checks do: it is
read by a program, and a path is either right or a loud line. Prose about *why* those are the files this
goal touches belongs in the goal's sections.

## 5. Pre-authorize every tradeoff, before the run

**Anything a session could reasonably stop and ask about will eventually hold the run on `BLOCKED`.** So
walk the path first and settle it with the user, then write each decision into the goal's
§ *Standing decisions*, as an instruction rather than a question: dependency choices, representation
choices, which language holes are in scope, which questions are closed and must not be re-opened.

The house rule inside that section: **decide and record, never `BLOCKED` for a design call.** A session
settles it under AGENTS.md's priority ordering, files it with `looper decision ask --built-on <n>`,
records it in the home AGENTS.md already names, and goes on. However large or hard to reverse the call
is, the session's recommendation is the answer until the user picks another. `BLOCKED` is reserved for
what the loop cannot do itself: a refused permission, a missing credential, or an act that publishes
outside the repository or costs money (AGENTS.md § *In the loop*).

Two things every standing decision should carry: the safe fallback if the implementation forces the
opposite conclusion, and where the answer gets written down. A decision with no home gets re-derived.

`looper lint` refuses a goal with no § *Standing decisions* heading, which is the cheap half of this and
the only half a tool can see. It cannot tell whether the section answers the questions the stages
actually reach — that is this section's judgement, and the goal whose rule is not yet written is where
it is hardest: implementing a settled rule asks a session to look a decision up, while writing one asks
it to make the decision at the moment it is least equipped to.

## 6. The stages that have always worked

Stages run in order, so the acceptance report names exactly how far the loop got. The shape that keeps
earning its place:

1. **Catch-up, if anything is owed** — decisions accepted *after* the goal that owns their work was
   reached. This runs before new breadth for one reason: every fixture written in the meantime is
   written against a rule that is about to change, and the rewrite compounds.
2. **A non-regression floor** — the checks every later goal keeps green, **never traded for anything
   above it.** This is what makes a long unattended run safe. In looper's format it is the `floor:` of
   `docs/goals/README.md`, and a check joins it only when it sets `floor: true`.
3. **The keystone**, if the goal has one — the single representation or mechanism everything else needs.
   If a fixture exists that cannot run until it lands, that fixture is the check.
4. **The breadth** — one fixture per domain, each reaching things no unit test proves reachable.
5. **The suites and the coverage gate** — see § 3 on why a count is not enough.
6. **The named guard tests**, then the leak sweep.

**Frozen output, unfrozen source.** A fixture's *expected output* may never be edited to make a check pass;
its *source* may be corrected freely, because whoever wrote it ahead of the code could not compile it. Say
in the commit message why a fixture was corrected. That distinction is what stops "make it pass" from
quietly becoming the goal.

## 7. Write the item list already grouped

The handoff names a **group** of slices sharing a file set, and a session takes as much of it as the
context ceiling allows. Order each stage's items so those groups fall out of the list, rather than
leaving them to be invented at 2 a.m. by a session that has not read the other items. The grouping is
what lets a session pick the slice whose files are cheapest to load.

Group by **the files an item touches, not its topic.** Two items in the same function are one group; two
items about the same rule in different crates usually are not. The shared file set is the entire mechanism:
it is what makes the second and third slice cost a fraction of the first. Where two adjacent items do not
share files, say so in the list — an item that gets its own session is a fine outcome, and pretending
otherwise costs a session a second orientation.

**Every item carries its anchors.** The rule that specifies it (`rule:<topic>/<rule>`), and the repo-rooted
`file.rs:NN` of the site it changes, which `looper lint` requires of every open item under `### Next`. You
are resolving them from context you already hold; a session without them spends ten `grep`s rediscovering
what you knew for free. This is also how the `context` gets written: the union of the anchors is the list.

**Then plan who does each piece.** Split a stage too big for one session, name its `**Split:**` and the
goal's lanes, gate each dependency on another goal, and settle every choice. The shape is
[conventions.md](conventions.md) § *A goal*. A goal that leaves these to its sessions pays for them
in every session.

## 8. What never goes in a loop

- **Doc trimming.** Nothing measures doc size ([doc-style.md](doc-style.md) § *Length targets*), and
  [doc-cleanup.md](doc-cleanup.md)'s pass is fired by the user, never from inside a run.
- **Dependency sweeps.** Also the user's to fire ([dependency-update.md](dependency-update.md)).
- **Backlog work.** If a slice is not on the path to the acceptance list, it goes in the handoff's
  `### Backlog` and the session moves on — **and if it will still be true after this goal ends it is a
  gap, so it is a record under `data/gaps/`** naming the module that owes it and the goal that owns it,
  where a goal switch cannot overwrite it. The handoff is state; a gap is not.
- **Re-opening a standing decision.** That is what § 5 exists to prevent.
- **Anything needing judgement about whether the goal is met.** The machine outranks the claim: looper
  runs the acceptance itself when a session reports `DONE`, and a goal is reached only when every stage
  is done and every check is green on the tree the session left.

## 9. When the run ends

Read the run's sessions again, as § 1 did before it: where they landed against the ceiling, and which
reads put them there. Only the second tells you what to change in the next goal's `context`. If the
evidence moved enough to change the ceiling or the strategy, change it **and say so in the commit** —
that is how the next goal starts from measurement rather than from whatever this file happened to say.
Fold anything durable the run taught you into the file that owns it: a trap into
[playbook.md](playbook.md), a shape into [conventions.md](conventions.md), a decision into the rule's
fragment under [docs/rules/](../rules/) with its reasons and its Declined list. The next goal is written for
its own target, never amended from this one's.
