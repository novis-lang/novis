# The parity program, as six loop goals

Orders 1–5 of [the plan](../../implementation-plan.md) are one continuous unattended run: **PHP core
feature parity, all five SQL drivers, concurrency, governance and the server.** This directory holds it,
cut into six goals, and [chain.toml](chain.toml) is the order the driver walks them in. A seventh,
post-parity goal — [ADR 0131](../../adr/0131-a-temporary-directory-dies-with-its-script-and-the-sweep-never-throws.md)'s
temporary-directory sweep — rides the same chain after the program's gate, because its server half needs
the `nvs-server` goal 6 creates. An eighth, [`Core\Program::id()`](8-program-id.md), follows it: one
member exposing the program fingerprint over hashes the artifact cache already computes. A ninth,
[`Core\Db\Schema`](9-schema.md), closes [ADR 0067](../../adr/0067-core-db.md)'s own *Revisiting* item and
sits there because its acceptance property needs every driver goal 5 builds to be finished. A tenth,
[a typed `callable`](10-typed-callable.md), follows it: [ADR 0136](../../adr/0136-a-callable-carries-its-signature.md)
gives the type a function value's parameters and return, and it goes after every goal that *writes*
callbacks so their registry rows are converted once rather than twice. An eleventh,
[doc comments](11-doc-comments.md), is last —
[ADR 0137](../../adr/0137-a-doc-comment-is-three-slashes-and-two-tags.md)'s `///`, whose stage 2 builds
ADR 0099 § 1's trivia layer because a doc comment cannot be read without it, so **M4B starts with its
own tree half already done**.

**Then M4B itself, as four goals** — [12 resilient-tree](12-resilient-tree.md),
[13 surface](13-surface.md), [14 lsp-server](14-lsp-server.md) and [15 editor](15-editor.md). Goal 13 is
not editor work: it is ADR 0098's pipeline operator and ADR 0124's PHP 8.6 refusals, the two M1 items
scheduled after M4 and never taken, and it sits before the grammar because a grammar written against a
surface about to change is written twice. Goal 15 is the last entry on the chain.

[loop-authoring.md](../loop-authoring.md) owns how a goal is *written* and [coordinator.md](../coordinator.md)
owns how one is *driven*. This file owns only what is specific to running six of them back to back, and it
does not restate either.

## Why six and not one

A goal is a finite contained group of work, and the `[context]` manifest is what keeps a session under the
200k ceiling. One goal spanning `nvs-syntax` through a TDS driver would need a manifest naming every
module in the workspace, and every byte of it is charged to every session — including the ones that never
open a driver. Six manifests, each naming the six-to-fifteen modules its own goal touches, is the same
work at a fraction of the per-session cost.

The split is **by file set, not by topic**. That is why M8 is two goals — `nvs-db` shares nothing with
`Core\Cli` — and why M5's reactor and its isolates are one, since both are `nvs-host`.

| Goal | Milestone | Crates it opens |
|---|---|---|
| [1 core-depth](1-core-depth.md) | M4S tail | `nvs-stdlib`, `nvs-types`, `nvs-hir`, `nvs-cli` |
| [2 concurrency](2-concurrency.md) | M5 | **`nvs-host`** (new), `nvs-runtime`, `nvs-stdlib` |
| [3 governance](3-governance.md) | M6 | **`nvs-config`** (new), `nvs-host`, `nvs-cli`, `nvs-codegen` |
| [4 core-part-ii](4-core-part-ii.md) | M8, non-database | `nvs-stdlib`, `nvs-host` |
| [5 database](5-database.md) | M8, database | **`nvs-db`** (new), `nvs-stdlib`, `nvs-types` |
| [6 server](6-server.md) | M7 | **`nvs-server`** (new), `nvs-stdlib`, `nvs-host` |
| [7 temp-sweep](7-temp-sweep.md) | post-parity, ADR 0131 | `nvs-runtime`, `nvs-host`, `nvs-stdlib`, `nvs-config`, `nvs-server`, `nvs-cli` |
| [8 program-id](8-program-id.md) | post-parity, ADR 0061 amendment | `nvs-config`, `nvs-hir`, `nvs-runtime`, `nvs-stdlib` |
| [9 schema](9-schema.md) | post-parity, one ADR slot | `nvs-db`, `nvs-stdlib`, `nvs-cli` |
| [10 typed-callable](10-typed-callable.md) | post-parity, ADR 0136 | `nvs-syntax`, `nvs-types`, `nvs-stdlib`, `nvs-ir`, `nvs-codegen`, `nvs-runtime` |
| [11 doc-comments](11-doc-comments.md) | post-parity, ADR 0137 + M4B's tree half | `nvs-syntax`, `nvs-diagnostics`, `nvs-hir`, `nvs-cli` |
| [12 resilient-tree](12-resilient-tree.md) | M4B, ADR 0099 § 1's other half | `nvs-syntax`, `nvs-diagnostics`, `nvs-test`, `nvs-cli` |
| [13 surface](13-surface.md) | M1 items 5-6, ADR 0098 + ADR 0124 | `nvs-syntax`, `nvs-diagnostics` |
| [14 lsp-server](14-lsp-server.md) | M4B, ADR 0099 §§ 3+5 + ADR 0101 | **`nvs-lsp`** (new), `nvs-cli`, `nvs-types`, `nvs-stdlib` |
| [15 editor](15-editor.md) | M4B, ADR 0099 §§ 4+6 | **`editors/vscode`** (new, TypeScript) |

## The chain contract

Each goal is three files, named for it:

| File | Holds |
|---|---|
| `N-<name>.md` | the target, the item list grouped by file set, the standing decisions |
| `N-<name>.toml` | the acceptance test as data, and the `[context]` manifest |
| `N-<name>.handoff.md` | the handoff the switch seeds, naming that goal's first group |

Three rules bind every one of them, and they are the reason the run can be left alone:

1. **A goal's acceptance list is the next goal's floor, mechanically.** `tools/goal-switch.py` copies every
   `[[check]]` out of the live `loop-goal.toml` into the next goal's own marker line, relabelled to the
   floor stage. Nothing is copied by hand. By goal 6 the floor is five goals deep, which is the point —
   the parity claim is only worth something if nothing under it was traded away to reach it.
2. **Every goal names the numbered ADRs it may open, and no session opens another.** The blanket "do not
   open a numbered ADR" rule that M4's goal carried does not survive this program: orders 2–5 contain
   genuinely new designs — a reactor, a driver's wire I/O, a pool reset that is a security boundary — and
   a design of that size recorded as a paragraph in `docs/adr/README.md` is a design nobody can find
   later. So each goal's *Standing decisions* carries a short list of **ADR slots**, each one the first
   slice of the goal that needs it. Anything not on that list is still decided-and-recorded, never
   `BLOCKED`, and never a new number.
3. **A goal that cannot verify itself does not advance.** The driver runs the acceptance test; a session
   claiming `DONE` against a red check stops the run, exactly as it does today.

## Starting the chain

Three steps.

1. Confirm the goal the repository is currently running is green: `python tools/loop.py --goal-only`.
   Whatever that goal is becomes goal 1's floor, so this is not a formality — it is the moment the floor
   is decided.
2. `python tools/loop-stats.py` and `python tools/loop-stats.py --attribute`.
   [loop-authoring.md](../loop-authoring.md) § 1 makes this step zero and § 9 says the numbers move. Set
   the slice budget from what it prints and **say which and why in the commit**. The 200k ceiling is not
   a number to re-derive; the *projection* is.
3. `python tools/loop.py --chain docs/agent/goals/chain.toml --max-sessions <n>`.

**The driver does the switching, including the first one.** It runs `tools/goal-switch.py` against the
entry it is about to install — which folds the live goal's whole acceptance list in as that entry's floor
— copies the three files into `docs/agent/loop-goal.md`/`.toml` and `docs/agent/handoff.md`, commits that
switch, and starts the session. On `GOAL REACHED` it does the same for the next entry and keeps going.
Without `--chain` the run stops six times and waits for a human, which is the same run with five extra
nights in it.

`.loop/chain.json` records which entry is installed, and it is what makes "exactly once per entry" a fact
rather than an intention: **`goal-switch.py` is not idempotent** — it inserts at a marker it leaves in
place, so running it twice inserts the floor twice. If you ever delete that state file, check the entry's
TOML for a doubled floor before restarting.

## What stops the run

- **The last goal goes green.** The parity program's own gate is goal 6's final check —
  `python tools/check-migration.py` reporting 100% classified — every one of the oracle build's **1151
  functions and 253 types** accounted for, every `member` row registered, every one of them cased. The
  inventory grew from 925 on 2026-08-29, when the oracle build gained `mysqli`, `pgsql` and `sqlite3`:
  the three APIs [ADR 0067](../../adr/0067-core-db.md) replaces are now inside the audit rather than a
  named hole beside it. Goals 7 through 11 going green, in chain order, is then what ends the run.
- **A goal reports `BLOCKED`.** Reserved for a decision that is expensive to reverse *and* has no safe
  default. Every goal's standing decisions exist to make this rare.
- **`--max-stalls` consecutive sessions move `HEAD` nowhere.**
- **Goal 5's Docker preflight fails.** ADR 0067 verifies the drivers against real servers, so the driver
  checks for a reachable daemon before the first session of that goal and stops the run naming it. A run
  that grinds for six hours against a check that cannot pass is worse than one that stops in the first
  minute.

## What this program does not touch

`Web\Migration` and everything versioned about a schema change — ordering, history tables, fleet
locking, reversibility — which [ADR 0082](../../adr/0082-the-first-party-framework.md) § 7 records as
deliberately blocked. Goal 9 builds convergence, which needs none of them, and does not close that gap.

Doc trimming and dependency sweeps, both of which the user fires and never a session
([doc-cleanup.md](../doc-cleanup.md), [dependency-update.md](../dependency-update.md)). And **PHP's
optional extensions** — `gd`, `intl`, `imap`, `zip` and the rest of the unaudited list in
[02-php-migration.md](../../spec/02-php-migration.md) — are not parity work: they are M9's, and a session
that finds one on its path puts it in the handoff's `## Backlog` and moves on.
