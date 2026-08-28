# The parity program, as six loop goals

Orders 1–5 of [the plan](../../implementation-plan.md) are one continuous unattended run: **PHP core
feature parity, all five SQL drivers, concurrency, governance and the server.** This directory holds it,
cut into six goals, and [chain.toml](chain.toml) is the order the driver walks them in.

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

Four steps. The third is the one that is easy to skip and expensive to skip.

1. Confirm M4's goal is green: `python tools/loop.py --goal-only`.
2. `python tools/loop-stats.py` and `python tools/loop-stats.py --attribute`.
   [loop-authoring.md](../loop-authoring.md) § 1 makes this step zero and § 9 says the numbers move. Set
   the slice budget from what it prints and **say which and why in the commit**. The 200k ceiling is not
   a number to re-derive; the *projection* is.
3. `python tools/goal-switch.py docs/agent/goals/1-core-depth.toml` — this folds M4's whole acceptance
   list into goal 1 as its floor. Doing it by hand is how a floor gets silently dropped, and a missing
   floor looks exactly like a passing one.
4. `python tools/loop.py --chain docs/agent/goals/chain.toml --max-sessions <n>`.

`--chain` is what makes the rest unattended: on `GOAL REACHED` the driver runs step 3 for the *next* goal
itself, copies it into `docs/agent/loop-goal.md`/`.toml`, seeds `docs/agent/handoff.md` from that goal's
`.handoff.md`, commits the switch, and starts the next session. Without it the run stops six times and
waits for a human, which is the same run with five extra nights in it.

## What stops the run

- **The last goal goes green.** That is the parity program finished, and the check that says so is
  `python tools/check-migration.py` reporting 100% classified — every one of the oracle build's 925
  functions and 240 types accounted for, every `member` row registered, every one of them cased.
- **A goal reports `BLOCKED`.** Reserved for a decision that is expensive to reverse *and* has no safe
  default. Every goal's standing decisions exist to make this rare.
- **`--max-stalls` consecutive sessions move `HEAD` nowhere.**
- **Goal 5's Docker preflight fails.** ADR 0067 verifies the drivers against real servers, so the driver
  checks for a reachable daemon before the first session of that goal and stops the run naming it. A run
  that grinds for six hours against a check that cannot pass is worse than one that stops in the first
  minute.

## What this program does not touch

Doc trimming and dependency sweeps, both of which the user fires and never a session
([doc-cleanup.md](../doc-cleanup.md), [dependency-update.md](../dependency-update.md)). M4B's staged goal
([next-goal-m4b.md](../next-goal-m4b.md)) stays unamended for order 6 of the milestone table. And **PHP's
optional extensions** — `gd`, `intl`, `imap`, `zip` and the rest of the unaudited list in
[02-php-migration.md](../../spec/02-php-migration.md) — are not parity work: they are M9's, and a session
that finds one on its path puts it in the handoff's `## Backlog` and moves on.
