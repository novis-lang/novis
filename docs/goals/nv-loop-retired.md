---
looper: 1
title: "The nv loop driver is gone, and looper runs the loop alone"
summary: "Looper runs this repository's loop, and no file of the old nv driver, nor any reader of its goal data, is left."
context: [looper.toml, tools/nv/main.ts, tools/nv/lib/chain.ts]
stages:
  - id: 1
    title: the first session under looper is the proof
    status: done
  - id: 2
    title: the readers
    status: done
    context: [tools/nv/cmd/affected.ts, tools/nv/cmd/guard.ts, .github/workflows/ci.yml]
  - id: 3
    title: the deletes
    status: done
    context: [tools/nv/cmd/audit.ts]
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
The runbook it followed is deleted, and `git log` holds it.

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

### 3. the deletes

The runbook's § 7: `tools/nv/driver/`, the seven loop commands, their `main.ts` entries and tests,
`renderers/goal-plan.ts` and `docs/agent/goal-plan.md`, `data/goals/`, `data/playbook/`,
`docs/agent/goals/`, and the prompts once AGENTS.md holds their rules. `lib/written.ts` stops writing
`NOVIS_LOOP_WRITES`, and the old driver's files under `.loop/` are deleted. Looper's own files there
stay.

The playbook bullets under `docs/agent/playbook/` still end on nv's `[until: <kind> <arg>]` trailer,
because the old driver reads only that form. Each one becomes looper's `` `until: <kind> <arg>` ``
trailer here, when nv's reader in `tools/nv/lib/playbook-fragments.ts` is deleted with the commands
that use it. Looper has no `rule` kind. A `rule <topic>/<slug>` trailer becomes
`exists docs/rules/<topic>/<slug>.md`, which holds when the same file exists.

### 4. the docs

`docs/agent/coordinator.md`, `loop-authoring.md` and the head of `playbook.md` are rewritten to point at
looper. Only text about the old driver itself is deleted. Then `docs/agent/looper-cutover.md` is
deleted, `bun nv links` is green, and so are `bun nv verify` and `looper lint`.

## Handoff

### State

**Stages 1, 2 and 3 are done. Stage 4's docs and tool comments are done. Its crate comments are
not.** `bun nv verify` is green on this tree.

- Landed: no comment under `tools/` names the old driver as if it ran, except `goalCopies` and
  `LEGACY` in `tools/nv/cmd/audit.ts`, which guard against its files and name them on purpose.
  `nv audit checks` no longer reads `docs/agent/loop-goal.toml`, which nothing writes. `lib/progress.ts`
  lost its `NV_PROGRESS_DIR` file mode and `readProgress`: only the old driver set or read them.
  `bun nv peek`'s ledger no longer reads `.loop/logs`, which looper never writes; the idle timer alone
  separates sessions. `.agent-tmp/verify-progress.json` stays, because the playbook bullet on a hung
  `verify --start` reads it.
- `docs/goals/decision-records-retired.md:542` quotes line spans of `loop-authoring.md` and
  `commands.md` that this group moved. That goal's first session re-derives them with a grep.
- `bun nv disk --clean` ran after every session under the old driver. Nothing runs it now, and
  `docs/agent/commands.md` says so. A looper hook for it would be a general feature.

**For the user: `.claude/settings.json` holds allow entries for deleted commands.** Its `bun nv`
entries for `chain`, `goal`, `loop`, `loop-stats`, `orient`, `playbook` and `session` name commands
that no longer exist. No session edits that file, so removing them is the user's change.

### Next

**Stage 4: the crate comments that name the old goal data** — doc comments only, so `bun nv verify`
rebuilds but no behaviour changes. A goal is now `docs/goals/<slug>.md` (`tools/nv/lib/chain.ts:1`).

- [x] Tool comments, `tools/nv/cmd/audit.ts`'s toml read, and the readers with no caller.
- [ ] **`crates/nvs-cli/tests/openapi.rs:394` and six more** say "the goal records under `data/goals/`",
  which becomes "a goal file under `docs/goals/`": `crates/nvs-db/src/catalog.rs:2426`,
  `crates/nvs-stdlib/src/mail.rs:1148`, `crates/nvs-stdlib/src/storage.rs:597`,
  `crates/nvs-stdlib/tests/tier_boundary.rs:36`, `crates/nvs-types/tests/visibility.rs:14`,
  `crates/nvs-stdlib/tests/spec_registry_coverage.rs:39`.
- [ ] **`crates/nvs-host/src/timer.rs:4` and 16 more files cite the deleted `docs/agent/goals/`**:
  `git grep -n docs/agent/goals/ -- crates Cargo.toml` lists 21 lines, among them `Cargo.toml`, `crates/nvs-stdlib/src/arr.rs`,
  `crates/nvs-ir/tests/type_atoms.rs` and four `crates/nvs-stdlib/tests/spec-*-outstanding.txt`.
  Restate what the citation pointed at, or drop it; never point at a goal file that is gone.
- [ ] `docs/rules/tooling/the-chain-names-its-live-goal.md:11`, the same citation in a rule.

### Backlog

- Stage 4: playbook bullets whose `until` names a deleted file, such as
  `tools/nv/cmd/orient.ts`; `looper playbook retire` deletes them once it runs.
