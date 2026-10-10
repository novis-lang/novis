---
looper: 1
title: "The nv loop driver is gone, and looper runs the loop alone"
summary: "Looper runs this repository's loop, and no file of the old nv driver, nor any reader of its goal data, is left."
context: [looper.toml, docs/agent/looper-cutover.md, tools/nv/main.ts, tools/nv/lib/chain.ts]
stages:
  - id: 1
    title: the first session under looper is the proof
    status: open
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
    name: no file under `tools/` imports the driver or a deleted loop command
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

**Stage 1 is blocked on the user: both its checks need a change to `.claude/settings.json`.**
`looper.toml`'s `[sync] skip` no longer lists that file. `bun nv verify` is green again: the handover's
deletion of `data/chain.json` had turned `tools/nv/test/surfaces.test.ts`'s `loop --list` test red,
and `crates/nvs-stdlib/tests/spec_registry_coverage.rs` panicked reading the file. That test now reads
the chain from `docs/goals/README.md`'s front matter (`chain()`), and the `loop --list` test is gone.

- `looper doctor` fails on `.claude/settings.json:201`, `Bash(bunx tsc:*)`: `bunx` is not on PATH here,
  and nothing in the tree runs it. `Bash(bun x tsc:*)` replaces it, and `looper sync` writes that entry.
- `looper lint` and `looper sync --check` fail because the handover wrote only `Bash(looper:*)`,
  `PowerShell(looper:*)` and the `looper guard` hook. Looper's template also wants its per-command
  `looper` entries, `bun install`, `bun run`, `bun x tsc`, `cargo run`, `cargo tree` and
  `cargo metadata`, for both shells.

**The user's fix:** run `looper sync`, delete the `Bash(bunx tsc:*)` line, and commit
`.claude/settings.json`. Then `looper doctor` and `looper lint` are expected to pass (*not checked*).

**For the user: `.claude/settings.json` loses entries in Stage 3.** Once the seven loop commands are
deleted, its `bun nv` allow entries for `chain`, `goal`, `loop`, `loop-stats`, `orient`, `playbook` and
`session` name commands that no longer exist. The runbook's § 4 lists their lines. No session edits that
file, so removing them is the user's change.

### Next

**Stage 1: the first session under looper is the proof** — one file set: `looper.toml`,
`docs/agent/looper-cutover.md`.

- [x] **The proof** — this session ran under looper, and `looper verify` is green.
- [x] **The `[sync] skip` line** for `.claude/settings.json` is deleted from `looper.toml`.
- [ ] **`looper doctor` and `looper lint`** — run both once the user's fix to `.claude/settings.json:201`
  above lands. Then set
  stage 1's status to `done`.

**Then Stage 2: the readers** — `docs/agent/looper-cutover.md:117-149` names each one. Still open
there: `crates/nvs-ir/tests/refusals.rs:95` (`HOLES_READS` lists `data/chain.json` and `data/goals`),
`tools/nv/lib/chain.ts`, and the tests that read the chain.

### Backlog

- Stage 2: the rest of the readers of the old goal data, and the playbook trailers.
- Stage 3: the deletes.
- Stage 4: the docs.
