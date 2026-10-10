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

**Stages 1, 2 and 3 are done; stage 4 is half done.** `bun nv links` and `looper lint` are green, and
`bun nv verify` passed on this tree.

- Landed: every dead link `bun nv links` named; AGENTS.md's Novis half names `looper orient`,
  `looper wrap` and `bun nv plan --apply` in place of the deleted `bun nv orient`, `session` and
  `loop-stats`; `docs/agent/coordinator.md` is rewritten around looper (design reasoning kept, the old
  driver's mechanics deleted); the head of `docs/agent/playbook.md` points at `looper wrap` and
  `looper playbook retire`; `docs/agent/looper-cutover.md` is deleted.
- `looper.toml` wires only `[verify] run`; no hook runs `bun nv sweep`, `services`, `legs` or `pack`.
  `docs/agent/commands.md:620-625` now says so.
- `carried-refusals.md` and `guard-name-debt.md` keep their `until` trailers, but `looper playbook
  retire` reads only the playbook, so nothing retires those entries on its own.

**For the user: `.claude/settings.json` holds allow entries for deleted commands.** Its `bun nv`
entries for `chain`, `goal`, `loop`, `loop-stats`, `orient`, `playbook` and `session` name commands
that no longer exist. No session edits that file, so removing them is the user's change.

### Next

**Stage 4: the rest of the process prose** — one file set: `docs/agent/loop-authoring.md` and the
other files under `docs/agent/` that name a deleted command.

- [ ] **`loop-authoring.md` points at looper** (`docs/agent/loop-authoring.md:3`), keeping its
  § 1 to § 9 numbering, which `docs/decisions/0099.md:117` and `docs/decisions/0134.md:152` cite.
  Driver text: `docs/agent/loop-authoring.md:3-14` (prose and record paths, `bun nv chain`), `:17-67`
  (`bun nv loop-stats`), `:73-190` (the `context` manifest as `bun nv orient` read it; looper's goal
  has `context` lists only), `:195-238` (`data/goals/`, `tools/nv/driver/console.ts`), `:311-338`.
- [ ] **Deleted commands in the other agent docs**: `docs/agent/commands.md:4`, `:25-31`, `:53`,
  `:66-89`, `:228`, `:248`, `:264`, `:294-312`, `:424-451`, `:493`, `:589`;
  `docs/agent/conventions.md:43`, `:437`, `:560`, `:606-630`; `docs/agent/doc-style.md:40-41`,
  `:51`, `:77-89`; `docs/agent/doc-cleanup.md:16`, `:57`; `docs/agent/dependency-update.md:29`;
  `docs/agent/optimization-prompt.md:41-42`, `:61`, `:122`, `:164`; `CONTRIBUTING.md:12`, `:90`.
  `docs/agent/commands.md` and `conventions.md` are in `looper.toml`'s `[sync] skip`, so they are
  edited by hand.

### Backlog

- Stage 4: crate doc comments that still say "the goal records under `data/goals/`"
  (`crates/nvs-cli/tests/openapi.rs:394`, `crates/nvs-db/src/catalog.rs:2426`,
  `crates/nvs-stdlib/src/mail.rs:1148`, `crates/nvs-stdlib/src/storage.rs:597`,
  `crates/nvs-stdlib/tests/tier_boundary.rs:36`, `crates/nvs-types/tests/visibility.rs:14`,
  `crates/nvs-stdlib/tests/spec_registry_coverage.rs:39`).
- Stage 4: tool comments and messages naming a deleted command: `tools/nv/cmd/pack.ts:1-12`,
  `tools/nv/cmd/brief.ts:21` and `:134`, `tools/nv/cmd/plan.ts:9` and `:317-318`,
  `tools/nv/cmd/site.ts:56` and `:290`, `tools/nv/lib/wrap.ts:2`, `tools/nv/cmd/disk.ts:777`,
  `tools/nv/accept/gates.ts:9`, `tools/nv/accept/sweep-lock.ts:18`, `tools/nv/accept/sweep.ts:3` and
  `:38-54`, `tools/nv/lib/proctree.ts:3`, `tools/nv/lib/progress.ts:6`, `tools/nv/proofs/run.ts:60`,
  `tools/nv/lib/machine.ts:18` ("The loop starts a background release build"),
  `tools/git-hooks/commit-msg:11`, `tools/leak-check.sh:3`. Check whether `applyPlan` in
  `tools/nv/cmd/plan.ts` and `wrapGate` in `tools/nv/cmd/site.ts` still have a caller.
- Stage 4: crate comments citing old goal prose under `docs/agent/goals/` (`git grep -n
  docs/agent/goals/ -- crates Cargo.toml` lists them), and
  `docs/rules/tooling/the-chain-names-its-live-goal.md:11`.
