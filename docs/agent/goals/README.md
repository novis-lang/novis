# The goal chain

This directory holds the prose of the goals the unattended loop walks, `data/chain.json` is their order,
and the table below is that order. A goal whose record says `position: last` stays behind every goal
that does not, and `bun nv chain --check` refuses an unpinned goal behind a pinned one.

**[goal-plan.md](../goal-plan.md) is the whole chain in one file**: every goal in order with its stages,
which goal is live, and the side goals. It is rendered from the records, and `bun nv chain`, the goal
switch and every wrap write it again when they change them.

**Every goal states its own case, and this file does not restate it.** A goal's front matter names its
milestone, its opening says what it builds, and its `## Why here` is why it sits where it does, against the
goals on either side. A paragraph here describing a goal would be a second copy of a fact that already has
a home, so there is none. [loop-authoring.md](../loop-authoring.md) owns how a goal is *written* and
[coordinator.md](../coordinator.md) owns how one is *driven*. What is left for this file is the contract
that binds the chain as a whole: why the work is cut into goals at all, what the three files are, the four
rules over them, and what stops the run.

## Why the work is cut into goals

A goal is a finite contained group of work, and its record's `context` is what keeps a session under the
200k ceiling. One goal spanning `nvs-syntax` through a TDS driver would need a manifest naming every
module in the workspace, and every byte of it is charged to every session — including the ones that never
open a driver. One manifest per goal, each naming the six-to-fifteen modules that goal touches, is the
same work at a fraction of the per-session cost.

The split is **by file set, not by topic**: two pieces of work that share no files are two goals, and
two that share a crate are one.

| Goal | Milestone | Crates it opens |
|---|---|---|
| [ldap](ldap.md) | M8, one new record — the `Core\Ldap` API — `position: last` | **`crates/nvs-ldap`** (new), `crates/nvs-stdlib/src/ldap/` (new), `nvs-config`, `tests/db/compose.yaml` — `Core\Ldap` over LDAPS and StartTLS with simple bind only, an immutable filter builder that encodes straight to BER, typed AD values, writes and passwords, `authenticate`, sort, VLV, deleted objects, DirSync and the Global Catalog, tested against Samba AD |
| [pdf](pdf.md) | M17, one new record — the `Novis\Pdf` API — `position: last` | **`extensions/pdf`** (new), `extensions/image`, `wit/`, `nvs-ext`'s build and load tables, `editors/vscode`'s custom data — `Novis\Pdf` built in: HTML and a documented CSS subset to tagged, byte-equal PDF with tables and headers that survive page breaks, PDF/A, Factur-X and passwords, placing, merging and reading existing PDFs; and SVG and PDF pages as image sources, shaping and barcodes in `Novis\Image` |
| [spreadsheet](spreadsheet.md) | M17, one new record — the `Novis\Spreadsheet` API — `position: last` | **`extensions/spreadsheet`** (new), `wit/`, `nvs-ext`'s build and load tables, `nvs-stdlib`'s zip bound — `Novis\Spreadsheet` built in: xlsx written as a stream in constant memory with styles, formulas, tables, conditional formats, images and charts, xlsx and xlsb read as a stream into typed values with xlsm, xls and ods beside them, designed templates filled with repeated rows, and CSV through the same API |

## The chain contract

**`data/chain.json` is the chain**: `goals`, a list of goal slugs whose order is the order the driver
walks, and `live`, the goal it works on. `live` is tracked in git, so every clone and CI see the
same live goal, and it is always first on `goals`. There is no second file saying
what the order is, so **reordering the chain is editing that list** — and `bun nv chain` is how that is
done, never by hand. `--new`, `--move` and `--remove` each edit `goals` in that one file and never
`live`, which only the driver's goal switch moves; `--check` says whether every goal is one the driver
can walk. `tools/nv/cmd/chain.ts`'s module doc is the tool's one home; this section is the contract it
enforces.

A goal the run has **not reached** may be inserted, edited or appended while the loop is running: every
turn of the driver is a fresh process that reads the chain again. The live goal's checks are rewritten
only by the session working it. A goal the run has reached is gone: the switch deleted it.

**Prose names a goal by its slug** — goal `html-later`, not goal 3. A number is a position, and it
moves the moment anything is inserted in front of it. `bun nv chain --check` fails on prose that names a
goal by its number. [AGENTS.md](../../../AGENTS.md) § *The schedule is the chain* is that rule's home.

Each goal is three files, named for it:

| File | Holds |
|---|---|
| `docs/agent/goals/<slug>.md` | front matter naming its milestone, the target, `## Why here`, the item list grouped by file set, the standing decisions |
| `data/goals/<slug>.json` | the acceptance test as data, and the `context` a session reads |
| `data/goals/<slug>.handoff.json` | the handoff, written with the goal to name its first group and rewritten by every session's wrap |

All three are deleted by the goal switch that leaves the goal. Four rules bind every one of them, and
they are the reason the run can be left alone:

1. **A goal's acceptance list is its own, and the suites are the floor.** A goal's plan is its record
   and nothing else (`goalPlan` in `tools/nv/lib/chain.ts`): no goal is in front of the live one, so no
   check is carried in. Finished work is protected by the permanent suites, `bun nv verify` and CI, so a
   check that proves more than a suite becomes a test before its goal is reached, because the check is
   deleted with the goal.
2. **Every goal names the numbered ADRs it may open, and no session opens another.** A goal can hold a
   genuinely new design — a reactor, a driver's wire I/O, a pool reset that is a security boundary — and
   a design of that size recorded as a paragraph in `docs/adr/README.md` is a design nobody can find
   later. So each goal's *Standing decisions* carries a short list of **ADR slots**, each one the first
   slice of the goal that needs it. Anything not on that list is still decided-and-recorded, never
   `BLOCKED`, and never a new number.
3. **A goal that cannot verify itself does not advance.** The driver runs the acceptance test; a session
   claiming `DONE` against a red check gets a retry session handed that check, and a retry that fails
   on the same check holds the run as `done-claim`.
4. **A goal switch deletes the goal it leaves.** It deletes the goal's prose, record and handoff record,
   removes its slug from `goals`, makes the next goal `live` and renders the goal plan, all in one
   commit. Nothing is archived, and `git log` is the history. A decision that only the goal's prose
   holds is moved to a rule, a record or a module doc before the goal is reached, and `bun nv chain
   --check` fails on prose outside a goal's own files that names a goal not on the chain. Decision
   records and [goal-decisions.md](../goal-decisions.md) are frozen history and exempt.
   `rule:tooling/the-chain-names-its-live-goal` is the rule.

## Side goals

**A side goal is a goal the chain never walks.** It is `docs/agent/goals/side/<slug>.md` and
`data/goals/side/<slug>.json` plus its handoff record beside that: the same three files, the same shapes
and the same rules as a chain goal, with three differences.

- **No place in the chain.** Nothing walks to it, so `data/chain.json` does not name it: its `.md`
  opens `# Side goal — <title>`, and prose names it by its slug as it names any goal. `bun nv chain
  --check` refuses a side goal with checks that has no prose, no such H1 or no handoff record.
- **A person starts its run by hand**, with `bun nv loop --side <slug>` typed in the main tree. It
  makes branch `side/<slug>` and its worktree at `.agent-tmp/worktrees/side/<slug>` when they are
  missing, and runs sessions there on that goal and nothing else. The driver never picks a side goal on
  its own. A side run and the chain run may run at once, because they share no tree, no build and no
  state file.
- **It lands instead of switching.** When its list is green the run ends on `SIDE GOAL GREEN` and
  prints the landing steps. The person rebases the branch onto `main`, runs `nv verify` and the whole
  list again, and fast-forwards `main` while the chain run holds between two sessions. **The goal is
  deleted as it lands**: one commit removes its prose, its record and its handoff, and the worktree and
  branch are removed after it. `bun nv chain --check` refuses a side goal with no checks, which is one
  that landed and was kept.

Its list is its own checks alone, and the permanent suites protect `main`'s finished work. The live
chain goal's own checks are not part of it. A side session writes no plan section and never edits the
chain run's files.

## Starting the chain

Three steps.

1. Confirm the goal the repository is currently running is green: `bun nv loop --goal-only`.
2. `bun nv loop-stats` and `bun nv loop-stats --attribute`.
   [loop-authoring.md](../loop-authoring.md) § 1 makes this step zero and § 9 says the numbers move. Set
   the slice budget from what it prints and **say which and why in the commit**. The 200k ceiling is not
   a number to re-derive; the *projection* is.
3. `bun nv loop`, typed by hand: it is the launcher [coordinator.md](../coordinator.md) § *Files*
   describes.

**The driver does the switching.** When a sweep is green and both goal-end gates are green — the
rustdoc gate `bun nv verify --doc`, and the owner gate `bun nv owners --closes <slug>` — it deletes the
goal, moves `live` in `data/chain.json` to the next goal and commits both as one commit, preflights and
brings up the new goal's `env.docker` services, and carries on with the next session. After the last
goal it ends the run on `CHAIN COMPLETE`. There is no flag for this and never a second chain: a run
that stopped at each green goal to wait for a human would be the same run with five extra nights in it.
A red gate holds the goal open, and `bun nv orient` prints the finding from `.loop/doc-gate.json` or
`.loop/owner-gate.json`.

## What stops the run

- **The last goal on `goals` goes green.**
- **A goal reports `BLOCKED`.** Reserved for a decision that is expensive to reverse *and* has no safe
  default. Every goal's standing decisions exist to make this rare.
- **`--max-stalls` consecutive sessions move `HEAD` nowhere.** The run gets a repair session first,
  and holds when that is spent.
- **A goal's Docker preflight fails.** `rule:core-classes/db-one-api` verifies the drivers against real
  servers, so the driver checks for a reachable daemon when a run starts on a goal with `env.docker`
  and at the switch to one, and stops naming it (`chain-error`, which gets a repair session and then
  holds). A run that grinds for six hours against a check that cannot pass is worse than one that stops
  in the first minute.

## What no goal on this chain takes

`Web\Migration` and everything versioned about a schema change — ordering, history tables, fleet
locking, reversibility — which `rule:programs/no-migration-runner` records as deliberately blocked.

Doc trimming, dependency sweeps and user reports, all of which the user fires and never a session
([doc-cleanup.md](../doc-cleanup.md), [dependency-update.md](../dependency-update.md),
[user-report.md](../user-report.md)); a report's outcome may be a goal, which is then written like any
other.
