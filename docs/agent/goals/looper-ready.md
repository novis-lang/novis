---
position: last
---
# Loop goal — Nothing outside the loop driver depends on it, and every hook looper will call is a `bun nv` command

Novis moves its loop onto looper, the external loop tool, and that move deletes `tools/nv/driver/` and
the seven loop commands — `tools/nv/cmd/{loop,chain,goal,session,orient,loop-stats,playbook}.ts`. This
goal makes the deletion safe and the move lossless, and it deletes nothing. When it is reached:

- **No file outside the driver and those seven commands imports them.** The code the rest of `tools/nv`
  shares with the driver lives in `tools/nv/accept/` and `tools/nv/lib/`, and a test proves the
  boundary.
- **Every piece of the driver that looper's hooks and checks will call is a `bun nv` command with its
  own tests**, and the driver calls the same code in-process, so each has one implementation:

| Command | What it does | Taken from |
|---|---|---|
| `bun nv suite [--case <path>]... [--min-passing <n>] -- <nvs args>` | runs one suite of `.nvst` cases and judges it as an `nvs-suite` check: every case green, each named case on disk and not skipped, at least `n` passing | `PlanSweep`'s `suite`/`judgeSuite`, `driver/runner.ts:580-613` |
| `bun nv legs [--goal <slug>]` | the Linux legs: the WSL build and mirror, the fixtures and suites again on Linux, the valgrind sweep | `linuxLegs` `driver/legs.ts:354`, `startWslBuild` `:320`, `syncMirror` `driver/mirror.ts:133` |
| `bun nv services --up [--goal <slug>]` | the Docker preflight and `docker compose up` of the goal's `env.docker` | `preflight` `driver/gates.ts:87`, `bringUp` `:110` |
| `bun nv sweep [--goal <slug>] [--done] [--keep-runs <n>]` | the sweep after a session: floor gate, full run, scoped and open sweeps, the bench turn on the gate, disk, doc and owner gates; writes `.loop/last-sweep.md`; always exits 0 | `serve()` `cmd/loop.ts:1271-1341` |
| `bun nv benches [--owed]` | the bench turn by hand, which `bun nv loop --benches [--owed]` is today | `benchTurn` `cmd/loop.ts:879`, `--benches` `:1475` |
| `bun nv side --prepare <slug> [--dir <path>]` | makes or completes a side worktree: branch, disk check, `bun install`, seeds, selection store; safe to repeat | `prepare` `driver/side.ts:61`, `seedStore` `:100` |
| `bun nv allow --learn <events.jsonl>` | records a finished session's tool calls and adds the owed allow rules | `AllowWatch` `driver/allow.ts:164`, `settle` `:287` |
| `bun nv plan --apply <wrap-file \| -> [--dry]` | checks, then writes, a wrap's `## plan:`, `## plan-edit:` and `## milestone:` sections | `validate` `cmd/session.ts:933`, `writeField` `:1323`, `applyPlan` `:1339`, `applyPlanEdit` `:1345`, `applyMilestone` `:1364` |
| `bun nv pack [--goal <slug>] [--stage <n>]` | prints the pack sections only Novis has: RULES, ADR SECTIONS, MAP scoped, SHAPES, PLAN FIELDS, MILESTONES, NEXT FREE NUMBER | `runRules` `cmd/orient.ts:719`, `runAdrs` `:773`, `runMap` `:953`, `runShapes` `:1026`, `runPlan` `:1298`, `runMilestones` `:1309`, `runNumbers` `:1345` |

- **`tools/nv/lib/written.ts` appends to the file `LOOPER_WRITTEN` names as well as the one
  `NOVIS_LOOP_WRITES` names**, so a looper session's written paths reach the same ledger readers.

The cutover itself is not part of this goal, because it deletes the driver that runs the loop.
[looper-cutover.md](../looper-cutover.md) is its runbook, and Stage 6 brings that runbook up to the
commands as built.

## Why here

The user decided to extend looper first and then move Novis onto it, losing nothing of the loop. The
looper-side work is written as looper goals. This goal is the Novis half, and it goes directly behind
goal `ldap`, the live goal, so the move does not wait for the rest of the chain. Goal
`decision-records-retired` comes after it and edits `cmd/orient.ts` and `cmd/session.ts`. This goal
moves code out of both, so it rewrites that goal's anchors to the new homes in the same slice
(Stage 5).

It carries `position: last` because every goal on the chain is pinned there.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check that goal `ldap` turned green stays
green, and `bun nv loop` keeps working at every commit: it is the program that runs this goal.

## Stage 2 — the shared code leaves the driver

**Does:** Moves the check model, the runner, the Linux legs, the origin, the sweep lock, `sweepOver` and `sweepIds` to `tools/nv/accept/` and the process tree to `tools/nv/lib/`, so nothing outside the driver and the seven loop commands imports them.

One file set: `tools/nv/driver/`, `tools/nv/accept/` (new), `tools/nv/lib/`, `tools/nv/cmd/loop.ts`,
`tools/nv/cmd/affected.ts`, `tools/nv/select/`, `tools/nv/proofs/run.ts`, `tools/nv/test/`.

- **The moves, with `git mv` and the file names kept:** `driver/proctree.ts` → `lib/proctree.ts`;
  `driver/{accept,runner,origin,sweep-lock,legs,mirror}.ts` → `accept/`. `sweepOver` (`cmd/loop.ts:417`)
  and `sweepIds` (`:560`) go to `accept/sweep.ts` with whatever of `loop.ts` they need; `sweepOver`
  calls `linuxLegs`, which is why the legs and the mirror move here and not in Stage 3.
- **The importers today**, every one re-pointed: `lib/proc.ts:13`, `lib/covws.ts:17` and its dynamic
  import at `:127`, `cmd/affected.ts:44-45` and its dynamic import of `sweepIds` at `:383`,
  `cmd/bench.ts:107`, `cmd/db-matrix.ts:72`, `cmd/try.ts:27`, `cmd/origin.ts:7`, `cmd/select.ts:28`,
  `proofs/run.ts:30`, `select/atoms.ts:19`, `select/checks.ts:47`, `select/full.ts:30-34`,
  `select/seed.ts:29`. `git grep -n -E "driver/|cmd/(loop|chain|goal|session|orient|loop-stats|playbook)\.ts|\./(loop|chain|goal|session|orient|loop-stats|playbook)\.ts" -- tools`
  lists them again.
- **Symbols a surviving file takes from a loop command move to where they belong:** `featureMatches`
  (`cmd/loop.ts:234`, used by `test/surfaces.test.ts:8`) to `accept/`; `NUMBER_CITE`
  (`cmd/chain.ts:55`, used by `test/render.test.ts:4`) to `lib/prose.ts`; `readSession`
  (`cmd/loop-stats.ts`, used by `test/surfaces.test.ts:9`) to a `lib/` module, or the part of that
  test that needs it moves to `driver/test/`.
- **A test follows the code it tests.** The tests of code that stays in `driver/` or in a loop command
  move to `tools/nv/driver/test/`: `launch`, `sweep`, `status`, `transcript`, `verdicts`, `side`,
  `allow`, `chain`, `goal`, `orient`, `playbook`, `session` and any other the boundary test names. `nvTestFiles` (`select/atoms.ts:75-93`) walks all of `tools/nv`, so they still run.
- **The boundary test**, `tools/nv/test/driver-boundary.test.ts`: *nothing outside the driver and the
  loop commands imports them* reads every `*.ts` under `tools/` outside `tools/nv/driver/` and the seven
  commands, takes each `from "…"` and `import("…")` specifier, and fails naming the file and the
  specifier that resolves into one of them. `main.ts`'s `COMMANDS` entries are the one exception. It
  also fails when it read fewer than 100 files, so a broken walk cannot pass. *The shared modules live
  outside the driver* fails while any moved file is still under `driver/` or `sweepOver` and `sweepIds`
  are still defined in `cmd/loop.ts`.
- **Pinned by** the Stage 2 check.

## Stage 3 — the check runners

**Does:** Adds `bun nv suite`, `bun nv legs` and `bun nv services`, and the driver calls their code.

One file set: `tools/nv/accept/`, `tools/nv/driver/gates.ts`, `tools/nv/cmd/loop.ts`, `tools/nv/main.ts`,
new `tools/nv/cmd/{suite,legs,services}.ts`, `tools/nv/test/`.

- **`bun nv suite`** builds a `Check` of kind `nvs-suite` from its arguments and runs it through
  `PlanSweep` (`accept/runner.ts`, `suite` at today's `driver/runner.ts:580`, `judgeSuite` at `:597`),
  so the verdict is the one a goal's check gets. It uses the binary the runner's build makes, and
  `NVS_BIN` when that names one. It prints the verdict's line and exits 0 on green, 1 on red.
- **`bun nv legs`** takes the goal's `env.wsl.targetDir` and `env.valgrind.skip` and the goal's
  fixtures and suites, as `sweepOver` does today when it builds `LegsOptions`, and runs `linuxLegs`.
  One function builds the options, and `sweepOver` and the command both call it. On a host where the
  legs do not apply it says so and exits 0.
- **`bun nv services --up`** runs `preflight` and then `bringUp` for the goal's `env.docker`, and both
  functions move out of `driver/gates.ts` into the command's module. The driver's calls at
  `cmd/loop.ts:1002`, `:1414` and `:1428` call them there. `memoizeOn` (`schema/goal.ts:61`) has no
  reader today, and this stage does not give it one.
- **Tests:** `test/suite.test.ts`, `test/legs-cmd.test.ts` and `test/services.test.ts`, each with the
  titles its check names, using the seams the moved code already has (`LegsSeams` in the legs, a docker
  runner passed in). *The check runners have one home* in the boundary test fails while `preflight` or
  `bringUp` is defined under `driver/`, or while `main.ts` does not register `suite`, `legs` and
  `services`.
- **Pinned by** the Stage 3 checks.

## Stage 4 — the session hooks

**Does:** Adds `bun nv sweep`, `bun nv benches`, `bun nv side --prepare` and `bun nv allow --learn`, and the driver calls their code.

One file set: `tools/nv/cmd/loop.ts`, `tools/nv/driver/{side,allow,gates}.ts`, `tools/nv/cmd/guard.ts`,
`tools/nv/proofs/select.ts`, `tools/nv/main.ts`, new `tools/nv/cmd/{sweep,benches,side,allow}.ts`,
`tools/nv/test/`.

- **`bun nv sweep`** is the sequence in `serve()` after a session, `cmd/loop.ts:1271-1341`, moved into
  one function that `serve()` calls: the `sweep: "done"` skip (`:1271`, the command's `--done` says the
  session reported DONE), the floor gate over the selection store's meta (`FLOOR_GATE_EVERY` `:181`,
  `HEAVY_FULL_EVERY` `:189`), `fullRunTurn` (`:858`), the scoped `sweepOver` and the open one
  (`:1301`, `:1317`), the bench turn on an open gate (`:1327`), `setFloorSince` (`:1330`), `sweepDisk`
  (`:1336`) and the doc and owner gates (`:1338-1340`). It writes the ledger lines it writes today, and
  `.loop/last-sweep.md`: one line per red check, its id, its name and its first failing line, or no
  file at all when the sweep is green. It exits 0 whatever it found. If the sweep cannot run, it writes
  why into `.loop/last-sweep.md` and still exits 0.
- **`bun nv benches [--owed]`** is `benchTurn` (`cmd/loop.ts:879`) as a command, and `bun nv loop
  --benches` (`:1475-1479`, the usage at `:166` and `:171`) is removed. The guard's heavy-run rule for
  it (`cmd/guard.ts:546`) moves to `benches` in the same slice.
- **`bun nv side --prepare <slug>`** is `prepare` (`driver/side.ts:61-93`) and `seedStore` (`:100`)
  as a command: branch `side/<slug>` from `main` and the worktree at `.agent-tmp/worktrees/side/<slug>`
  when they are missing, or the worktree at `--dir`, then the disk check, `bun install`, the seeds and
  the selection store, each only when missing. `launchSide` (`:116`) calls it. `landing`,
  `insideWorktree` and `launchSide` stay in the driver.
- **`bun nv allow --learn <events.jsonl>`** feeds a finished session's stream-json events to
  `AllowWatch` (`driver/allow.ts:164`), records the outcome (`record`, `:215`) and then runs `settle`
  (`:287`), which the driver runs today before the next launch (`cmd/loop.ts:1112`, `:1138`,
  `:1162-1163`). `AllowWatch`, `record`, `owed`, `nvCommands` and `settle` move into the command's
  module, and the driver imports them from there.
- **Tests:** `test/after-session.test.ts`, `test/side-prepare.test.ts`, `test/allow-learn.test.ts` and
  `test/benches.test.ts`, each with the titles its check names; the sweep's tests pass a runner in and
  start no cargo. *The session hooks have one home* fails while `prepare`, `seedStore`, `AllowWatch`,
  `settle` or `benchTurn` is defined under `driver/` or in a loop command, while `cmd/loop.ts` still
  parses `--benches`, or while `main.ts` does not register `sweep`, `benches`, `side` and `allow`.
- **Pinned by** the Stage 4 checks.

## Stage 5 — the wrap sections, the pack sections and the written paths

**Does:** Adds `bun nv plan --apply` and `bun nv pack`, which `nv session --wrap` and `nv orient` call, and `lib/written.ts` also writes `LOOPER_WRITTEN`.

One file set: `tools/nv/cmd/session.ts`, `tools/nv/cmd/plan.ts`, `tools/nv/cmd/orient.ts`, new
`tools/nv/cmd/pack.ts`, `tools/nv/lib/written.ts`, `tools/nv/main.ts`, `tools/nv/test/`,
`docs/agent/goals/decision-records-retired.md`.

- **`bun nv plan --apply`** reads a wrap file, or one section on stdin with `-`, and takes only its
  `plan`, `plan-edit` and `milestone` sections (`KNOWN`, `cmd/session.ts:105`). It runs those
  sections' part of `validate` (`:933`, the plan arms at `:950`, `:974` and `:1003`) and then, without
  `--dry`, `writeField`, `applyPlan`, `applyPlanEdit` and `applyMilestone` (`:1323-1364`). Those move
  into `cmd/plan.ts`, and `applyWrap` (`:1565`) calls them there. `plan.ts`'s summary (`:19`) names
  `--apply` and stops naming `--stale`, which `run` does not handle.
- **`bun nv pack`** prints, for one goal and one stage, the sections `bun nv orient` prints that looper
  has no form for: RULES, ADR SECTIONS, MAP scoped, SHAPES, PLAN FIELDS, MILESTONES and NEXT FREE
  NUMBER, in `orient`'s order. The `run*` functions above and the manifest they read (`manifest`,
  `cmd/orient.ts:348`) move into `cmd/pack.ts`, and `orient` calls them there. TRAPS stays in `orient`:
  the playbook moves to looper's own format at the cutover. The goal is `--goal`, else `LOOPER_GOAL`,
  else the chain's live goal; the stage is `--stage`, else `LOOPER_STAGE`, else the handoff's next
  stage.
- **`lib/written.ts`** (`ENV` at `:9`) appends each path to the file `NOVIS_LOOP_WRITES` names and to
  the file `LOOPER_WRITTEN` names, each only when it is set.
- **Goal `decision-records-retired`'s anchors.** Its Stage 12 cites `orient.ts` and `session.ts` lines
  (`docs/agent/goals/decision-records-retired.md:187-198`), and some of that code moves here. The slice
  that moves it rewrites each such anchor to the new file and line, in that goal's prose and handoff.
- **A side run still may not write the plan** (`cmd/session.ts:943`): `--apply` keeps that refusal
  when the tree is a side worktree.
- **Tests:** `test/plan-apply.test.ts`, `test/pack.test.ts` and `test/written.test.ts`, each with the
  titles its check names. *The wrap and orient sections have one home* fails while any of
  `writeField`, `applyPlan`, `applyPlanEdit`, `applyMilestone`, `runRules`, `runAdrs`, `runMap`,
  `runShapes`, `runPlan`, `runMilestones` or `runNumbers` is defined in `cmd/session.ts` or
  `cmd/orient.ts`, or while `main.ts` does not register `pack`.
- **Pinned by** the Stage 5 checks.

## Stage 6 — the runbook names the commands

**Does:** Brings `docs/agent/looper-cutover.md` and `docs/agent/commands.md` up to the commands as built, and a test holds the runbook's hook table to `bun nv help`.

One file set: `docs/agent/looper-cutover.md`, `docs/agent/commands.md`, `tools/nv/test/driver-boundary.test.ts`.

- The runbook's hook table names each command with the flags it has. Where a flag changed while it was
  built, the table and the command's `--help` agree.
- `commands.md` gets one entry per new command, in the shape its neighbours have.
- *Every command the cutover runbook names is registered* reads the `bun nv <command>` tokens in the
  runbook's hook table and fails on one that `main.ts`'s `COMMANDS` does not hold.
- **Pinned by** the Stage 6 check.

## Standing decisions

- **The user's calls, as instructions.** Looper is extended first, then Novis moves onto it, and nothing
  of the loop is lost: not the main chain, not a side goal. A gap is closed in looper by a general
  feature, never by a fork or a Novis-only field in looper's goal format. This goal is the Novis half,
  and it deletes nothing that the driver needs.
- **The driver works at every commit.** `bun nv loop`, `--side`, `--goal-only`, `--list` and the goal
  switch behave as before, except that the bench turn by hand is `bun nv benches`. If a move breaks
  the driver in a way the slice cannot fix, the slice is not committed, and the session says why in
  the handoff. The driver may import from the new homes. A new home never imports from the driver.
- **The goal writer's calls, not confirmed by the user, also standing.**
  - The shared code goes to `tools/nv/accept/` and the process tree to `tools/nv/lib/proctree.ts`. Files
    keep their names, so `git log --follow` reads straight. If a session finds a better split, it
    records it in the module's `//!` header. The tests do not name the directory, only "outside the
    driver".
  - The command names and flags are the table at the top. A flag may be added while a command is built.
    A name changes only with the runbook's hook table in the same slice.
  - A command reads a goal's record only through `lib/chain.ts`, so the cutover re-points one module.
    `--goal` and `--stage` win. `LOOPER_GOAL` and `LOOPER_STAGE` come next, because those are the names
    the looper side was asked for. If looper picks other names, the cutover passes flags in the hook's
    argv, and no session waits on looper.
  - `bun nv suite` takes its case list on the command line, not a check id, so looper's importer can
    write one argv per `nvs-suite` check and the command needs no goal record.
  - `bun nv sweep` always exits 0, because looper pauses on a hook that fails. A green sweep leaves no
    `.loop/last-sweep.md`, because looper's orient prints that file under "Fix these first" whenever it
    is not empty.
  - The floor gate, the heavy-check hold and the bench cadence stay inside `bun nv sweep`, kept in the
    selection store's meta as today. Looper needs no cadence field for Novis.
  - `bun nv allow --learn` takes a file path. The runbook wires looper's session log to it.
  - TRAPS is not in `bun nv pack`.
- **Exit codes:** 0 green, 1 red, 2 a usage error, for every new command except `bun nv sweep`.
- **Nothing here calls looper, reads `looper.toml` or edits `E:\loop-plan-tool`.** The goal record's
  schema gets no new field.
- **`.claude/settings.json` is not edited by a session.** The driver's allow learning adds the
  `bun nv <command>` entries for a new command on its own (`nvCommands`, `driver/allow.ts:262`).
- **The tradeoffs**, as AGENTS.md asks. The language, its performance and its memory do not change. For
  agents: `bun nv help` gains eight commands, each one does one job a person or a hook can run alone, and
  the driver calls the same code in-process, so a session costs what it costs today. What looper's
  hooks will call is tested before the cutover, not discovered during it.
