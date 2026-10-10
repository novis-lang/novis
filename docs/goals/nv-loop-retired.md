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

**Stages 1, 2 and 3 are done.** No file of the old driver, its loop commands, its goal data or its
prompts is left, and `no-import-of-the-driver` is green. What stays is prose that still describes them.

- `docs/agent/goals/`, `session-prompt.md` and `repair-prompt.md` are deleted. Their rules a looper
  session still needs are in AGENTS.md: the handoff's shape in § *Keep each slice small*, side goals
  under looper in § *The schedule is the chain*, and the rest in the new § *In the loop* (permission
  denials, delegated searches, `verify --start`, `BLOCKED`, the goal-end gates, a repair's bar). The
  side goals' old prose differed from `docs/goals/compare-page.md` and `search-questions.md` only in
  its headings.
- `bun nv audit goals` now flags any tracked file under `docs/agent/goals/` as a copy, and reads goal
  numbers under `docs/goals/` alone; the live goal's prose it skips is `docs/goals/<live>.md`
  (`cmd/audit.ts:47-55`, `:250`). `LEGACY`'s `docs/agent/goals/` pattern stays, since it names old
  paths as history.
- The old driver's code, data and `.loop/` files went in earlier sessions; `git log` has the list.

**For the user: `.claude/settings.json` holds allow entries for deleted commands.** Its `bun nv` entries
for `chain`, `goal`, `loop`, `loop-stats`, `orient`, `playbook` and `session` (lines 32-70 and 115-153)
name commands that no longer exist. No session edits that file, so removing them is the user's change.

### Next

**Stage 4: the docs, process prose first** — one file set: `AGENTS.md`, `docs/agent/*.md`,
`CONTRIBUTING.md`. The commit is `docs(agent): point the loop rules at looper`.

- [ ] **AGENTS.md's Novis half names deleted commands.** `bun nv orient` at `AGENTS.md:21-22` and
  `:269`, `bun nv session --wrap` at `AGENTS.md:101` and `:298-301`, `nv session` at `AGENTS.md:31`,
  `bun nv loop-stats` at `AGENTS.md:283`. Each becomes looper's command or goes; the looper block
  (`AGENTS.md:356` on) is `looper sync`'s and is not edited by hand.
- [ ] **Dead links to the deleted prompts and goals README**: `docs/agent/coordinator.md:21`, `:28`,
  `:42`, `:80`, `:222`, `:419`, `:493`, `:550`, `:562`; `docs/agent/user-report.md:85` and `:119`;
  `CONTRIBUTING.md:203-205`; `docs/agent/optimization-prompt.md:107`. A side-goal link goes to
  AGENTS.md § *The schedule is the chain*. `bun nv links` names 10 dead links, these with
  `docs/agent/carried-refusals.md:28`, `docs/agent/guard-name-debt.md:80` and
  `docs/decisions/0130.md:129`.
- [ ] **`coordinator.md`, `loop-authoring.md` and `docs/agent/playbook.md:5-6`** rewritten to point at
  looper, per § *Standing decisions*; then `docs/agent/looper-cutover.md` is deleted and `bun nv links`
  run.

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
- Stage 4: crate comments citing old goal prose under `docs/agent/goals/` (`Cargo.toml:147`,
  `crates/nvs-ir/tests/refusals.rs:24` and `:302`, `crates/nvs-ir/tests/type_atoms.rs:40`, and more;
  `git grep -n docs/agent/goals/ -- crates Cargo.toml` lists them), and
  `docs/rules/tooling/the-chain-names-its-live-goal.md:11`.
