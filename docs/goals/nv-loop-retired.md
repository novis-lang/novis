---
looper: 1
title: "The nv loop driver is gone, and looper runs the loop alone"
summary: "Looper runs this repository's loop, and no file of the old nv driver, nor any reader of its goal data, is left."
context: [looper.toml, docs/agent/looper-cutover.md, tools/nv/main.ts, tools/nv/lib/chain.ts]
stages:
  - id: 1
    title: the first session under looper is the proof
    status: done
  - id: 2
    title: the readers
    status: open
    context: [tools/nv/cmd/affected.ts, tools/nv/cmd/guard.ts, tools/nv/lib/playbook-fragments.ts, .github/workflows/ci.yml]
  - id: 3
    title: the deletes
    status: open
    context: [tools/nv/driver/handover.ts, tools/nv/lib/written.ts]
  - id: 4
    title: the docs
    status: open
    context: [docs/agent/coordinator.md, docs/agent/loop-authoring.md, docs/agent/playbook.md]
checks:
  - id: looper-doctor
    stage: 1
    name: "`looper doctor` finds the tree ready for looper"
    run: [looper, doctor]
  - id: no-import-of-the-driver
    stage: 3
    name: "no file under `tools/` imports the driver or a deleted loop command"
    run: [git, grep, -l, -E, "(driver/|cmd/(loop|chain|goal|session|orient|loop-stats|playbook))[a-z/-]*\\.ts", --, tools/]
    exit: nonzero
  - id: links
    stage: 4
    name: no Markdown link points at a deleted file
    run: [bun, nv, links]
  - id: verify
    stage: 4
    name: "`bun nv verify` passes"
    run: [bun, nv, verify]
  - id: lint
    stage: 4
    name: "`looper lint` finds no problem"
    run: [looper, lint]
---

# The nv loop driver is gone, and looper runs the loop alone

## Why

### The target

Goal `looper-cutover` moved every goal, both side goals and the playbook into looper's format, and the
old driver's handover started looper's loop. The old driver's code, its goal data and every reader of
that data are still in the tree. This goal deletes them, so looper is the only loop this repository has.
The runbook is `docs/agent/looper-cutover.md`, and its § 5 and § 7 name each file.

## Standing decisions

- **Nothing of the loop is lost.** A reader of the old data is re-pointed at looper's goal files, or
  deleted together with what it serves. A gap is closed by a general looper feature, never by a fork or
  a Novis-only field.
- **`.claude/settings.json` is not edited by a session.** The handover wrote what looper needs. The
  entries the runbook's § 4 says it loses are listed in this goal's handoff for the user.
- **The side worktrees under `.agent-tmp/worktrees/side/` are never touched**, nor their branches or
  their `.loop/`.
- **The prompts.** `[prompts]` stays unset, so looper's built-in prompts run. AGENTS.md holds every rule
  a looper session needs before the prompts are deleted.
- **`coordinator.md`, `loop-authoring.md` and the head of `playbook.md`** are rewritten to point at
  looper. Only text about the old driver itself is deleted.

## Stages

### 1. the first session under looper is the proof

The runbook's § 7 steps 2 and 3. The first session under looper commits its wrap, and its status line is
read. A sweep or an orient that fails is fixed here first. This stage's check is `looper doctor`, which
passes once the handover has written the `looper guard` hook.

`looper.toml`'s `[sync] skip` lists `.claude/settings.json`, because goal `looper-cutover`'s `looper lint`
check had to pass before the handover wrote looper's entries there. That line is deleted here, and
`looper lint` then compares the file again.

### 2. the readers

Every reader of `data/chain.json`, `data/goals/` and `data/playbook/` that the runbook's § 5 names is
re-pointed at looper's goal files, or deleted with what it serves. That includes `lib/chain.ts`,
`bun nv plan --check`, `bun nv render --check`, `bun nv check`'s four schemas, `cmd/affected.ts`'s
session base and `cmd/guard.ts`'s goal rules. CI's `chain --check` and `playbook --check` lines go,
because CI has no looper.

The playbook bullets under `docs/agent/playbook/` still end on nv's `[until: <kind> <arg>]` trailer,
because the old driver reads only that form. Each one becomes looper's `` `until: <kind> <arg>` ``
trailer here, once nv's reader in `tools/nv/lib/playbook-fragments.ts` is gone. Looper has no `rule`
kind. A `rule <topic>/<slug>` trailer becomes `exists docs/rules/<topic>/<slug>.md`, which holds when
the same file exists.

### 3. the deletes

The runbook's § 7: `tools/nv/driver/`, the seven loop commands, their `main.ts` entries and tests,
`renderers/goal-plan.ts` and `docs/agent/goal-plan.md`, `data/goals/`, `data/playbook/`,
`docs/agent/goals/`, and the prompts once AGENTS.md holds their rules. `lib/written.ts` stops writing
`NOVIS_LOOP_WRITES`, and the old driver's files under `.loop/` are deleted. Looper's own files there
stay.

### 4. the docs

`docs/agent/coordinator.md`, `loop-authoring.md` and the head of `playbook.md` are rewritten to point at
looper. Only text about the old driver itself is deleted. Then `docs/agent/looper-cutover.md` is
deleted, `bun nv links` is green, and so are `bun nv verify` and `looper lint`.

## Handoff

### State

**Stage 1 is done.** The user's sync of `.claude/settings.json` landed, and `looper doctor` and
`looper lint` both pass.

**Stage 2 has started: `tools/nv/lib/chain.ts` reads looper's goal files.** The chain is the `chain:`
list of `docs/goals/README.md`, and its first slug is live. A goal's plan comes from its front matter
through `planOf`: Novis's own fields come from `extra`, and each check's kind follows from its fields
(`checkOf`). A side goal is a goal file off the chain. `bun nv pack` takes its default stage from the
goal's first open stage (`liveStage`). Every command that imports `lib/chain.ts` now reads looper's
files, so `bun nv plan --check` passes again. Its four Carried by cells and `Open now` were fixed to
match.

The old driver's writes to its own data (`setLive`, `leaveGoal`, `goalFiles`, `oldLive`) moved to
`tools/nv/driver/old-chain.ts`. Their only users are `cmd/loop.ts`, `cmd/goal.ts`,
`driver/handover.ts` and the tests under `driver/test/`, and Stage 3 deletes all of them.
`docs/agent/goal-plan.md` was re-rendered so `bun nv render --check` passes, and Stage 3 deletes it with
its renderer.

**For the user: `.claude/settings.json` loses entries in Stage 3.** Once the seven loop commands are
deleted, its `bun nv` allow entries for `chain`, `goal`, `loop`, `loop-stats`, `orient`, `playbook` and
`session` name commands that no longer exist. The runbook's § 4 lists their lines. No session edits that
file, so removing them is the user's change.

### Next

**Stage 2: the readers.** Files: `tools/nv/cmd/holes.ts`, `crates/nvs-ir/tests/refusals.rs`,
`tools/nv/cmd/affected.ts`, `tools/nv/cmd/guard.ts`, `tools/nv/schema/index.ts`, `.github/workflows/ci.yml`.

- [x] **`lib/chain.ts`** reads looper's goal files, and its consumers follow.
- [ ] **`cmd/holes.ts:280`** names `data/goals/<slug>.json` as the live goal's path, and its head comment
  at `:31` says the same. Both become `docs/goals/<slug>.md`.
- [ ] **`crates/nvs-ir/tests/refusals.rs:95-96`**: `HOLES_READS` lists `data/chain.json` and
  `data/goals`. It lists `docs/goals` instead.
- [ ] **`cmd/affected.ts:57`** (`SESSION_BASE`, `.loop/session-start.json`) and `:119-130` take the
  session base from looper's session base. `LOOPER_WRITTEN` says a loop session is running.
- [ ] **`cmd/guard.ts:319`** (`GOAL_CHECKS`) and the `goal-grep` rule at `:371`, and the
  `loop --goal-only|--run` refusal, move to `docs/goals/*.md` and looper's commands.
- [ ] **`bun nv check`'s four schemas**: `schema/index.ts:10-17` registers `chain`, `goal`/`sideGoal`,
  `handoff`/`sideHandoff` and the playbook records. They go with their data in Stage 3, so CI's
  `bun nv check` (`ci.yml:915`) stops validating them.
- [ ] **CI**: `ci.yml:939` (`bun nv chain --check`) and `:941` (`bun nv playbook --check`) are deleted,
  because CI has no looper.
- [ ] **`tools/nv/test/registers-reads.test.ts:11-16`** reads `data/playbook/`.
- [ ] **The playbook trailers**, read at `tools/nv/lib/playbook-fragments.ts:19` (`EXPIRY`): every
  bullet under `docs/agent/playbook/` still ends on nv's `[until: <kind> <arg>]`. Each one becomes
  looper's `` `until: <kind> <arg>` `` once that reader stops reading the old form. A `rule <topic>/<slug>` trailer
  becomes `exists docs/rules/<topic>/<slug>.md`.

### Backlog

- Stage 3: the deletes, among them `tools/nv/driver/` with `old-chain.ts`, the seven loop commands, and
  `renderers/goal-plan.ts` with `docs/agent/goal-plan.md`.
- Stage 4: the docs.
