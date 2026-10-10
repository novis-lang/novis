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
    status: done
    context: [tools/nv/cmd/affected.ts, tools/nv/cmd/guard.ts, .github/workflows/ci.yml]
  - id: 3
    title: the deletes
    status: open
    context: [docs/agent/goals/README.md, docs/agent/session-prompt.md, tools/nv/cmd/audit.ts]
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

**Stages 1 and 2 are done, and Stage 3's code deletes have landed.** The check
`no-import-of-the-driver` is green: `git grep` finds no file under `tools/` that names the driver or a
deleted loop command.

- `tools/nv/driver/`, its tests, the seven loop commands and their `main.ts` entries are gone, with
  `lib/handoff.ts`, `test/driver-boundary.test.ts` and `accept/accept.ts`'s `featureMatches`, whose only
  caller was `loop --list`. `test/surfaces.test.ts` covers `brief --where` and `guard --check` alone.
- `schema/index.ts` registers ten types. The `chain`, `goal`, `handoff` and playbook schemas,
  `data/goals/`, `data/playbook/` and their JSON Schemas under `data/schema/` are deleted, and so are
  `renderers/goal-plan.ts`, `docs/agent/goal-plan.md` and `lib/playbook-fragments.ts`. A gap's `goal` is
  now a plain slug (`schema/gap.ts:17`), and `bun nv owners` checks it against `docs/goals/`.
  `test/no-hand-build.test.ts` reads every goal file's checks through `lib/chain.ts`.
- Every bullet under `docs/agent/playbook/` ends on looper's `` `until: <kind> <arg>` `` trailer. None
  used the `rule` kind. A trailer whose argument held backticks was cut to text without them, because
  looper ends the trailer at the next backtick (`crates/looper-core/src/playbook.rs:98` in looper's tree).
  The many bullets whose trailer says `gone` of a deleted file now hold their condition, so
  `looper playbook retire` deletes them when it runs.
- `lib/written.ts` writes only `LOOPER_WRITTEN`'s ledger.
- `bun nv side --prepare` looks for the side goal at `docs/goals/<slug>.md` (`cmd/side.ts:88`).
- The old driver's files under `.loop/` are deleted: `chain.json`, `run.json`, `session-start.json`,
  `written.txt`, `pack-size.jsonl`, `progress/` and `logs/`. The rest stays, because a live tool or
  looper writes it, or its writer is *not checked*: `dossier-*`, `optimization/`, `optimize-status.txt`,
  `release-gate.json`, `state.sqlite` and `test.nvs`.

**For the user: `.claude/settings.json` holds allow entries for deleted commands.** Its `bun nv` entries
for `chain`, `goal`, `loop`, `loop-stats`, `orient`, `playbook` and `session` (lines 32-70 and 115-153)
name commands that no longer exist. No session edits that file, so removing them is the user's change.

### Next

**Stage 3's last item, then the stage is done.** Files: `docs/agent/goals/` (`README.md` and
`side/{compare-page,search-questions}.md`), `docs/agent/session-prompt.md`, `docs/agent/repair-prompt.md`,
`AGENTS.md`, `tools/nv/cmd/audit.ts`.

- [x] **The driver and the seven loop commands**, with `lib/handoff.ts`.
- [x] **The record types and their data together**, with their tests and JSON Schemas.
- [x] **`renderers/goal-plan.ts`** and `docs/agent/goal-plan.md`.
- [x] **The playbook trailers**, in looper's form.
- [x] **`lib/written.ts`** writes looper's ledger alone, and the old driver's files under `.loop/` are deleted.
- [ ] **`docs/agent/goals/` and the prompts** (`docs/agent/looper-cutover.md:176`), once AGENTS.md
  holds their rules. First compare each rule of `session-prompt.md` and `repair-prompt.md` with
  AGENTS.md § *Session workflow* and § *The context budget*, and move into AGENTS.md what it lacks.
  `docs/agent/goals/side/*.md` duplicate `docs/goals/compare-page.md` and `search-questions.md`; check
  that nothing in them is missing there before deleting. `cmd/audit.ts:36` (`GOALS`), `:48`, `:57`,
  `:218` and `:255` still name `docs/agent/goals/`: they recognise its old paths as history, so decide
  whether each test stays as a legacy pattern or goes, with `test/audit.test.ts:27-176`.
  `optimization-prompt.md` is not a loop prompt and stays.

### Backlog

- Stage 4: the docs. The crate doc comments that still say "the goal records under `data/goals/`"
  (`crates/nvs-cli/tests/openapi.rs:394`, `crates/nvs-db/src/catalog.rs:2426`,
  `crates/nvs-stdlib/src/mail.rs:1148`, `crates/nvs-stdlib/src/storage.rs:597`,
  `crates/nvs-stdlib/tests/tier_boundary.rs:36`, `crates/nvs-types/tests/visibility.rs:14`,
  `crates/nvs-stdlib/tests/spec_registry_coverage.rs:39`) are rewritten with them.
- Stage 4: the tool comments and messages that still name a deleted command:
  `cmd/pack.ts:1-12` (says `orient` calls its `run*` functions), `cmd/brief.ts:21` and `:134`,
  `cmd/plan.ts:9` and `:317-318`, `cmd/site.ts:56` and `:290`, `lib/wrap.ts:2`, `cmd/disk.ts:777`,
  `accept/gates.ts:9`, `accept/sweep-lock.ts:18`, `accept/sweep.ts:3` and the `nv loop:` messages at
  `:38-54`, `lib/proctree.ts:3`, `lib/progress.ts:6`, `proofs/run.ts:60`, `tools/git-hooks/commit-msg:11`
  and `tools/leak-check.sh:3`. `cmd/plan.ts`'s `applyPlan` and `cmd/site.ts`'s `wrapGate` were
  `bun nv session --wrap`'s, so check whether each still has a caller.
- Stage 4: `docs/agent/playbook.md:5-6` still describes the `[until: …]` form and `cmd/playbook.ts`.
