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

**Stages 1 and 2 are done.** Every reader of the old goal data that is not deleted in Stage 3 now reads
looper's goal files through `tools/nv/lib/chain.ts`, which exports `goalPaths` for a command that needs
every goal file, on the chain or off it.

- `bun nv holes` reads the live goal's items and the cases its checks name from `docs/goals/<slug>.md`
  (`cmd/holes.ts:279`), and `HOLES_READS` lists `docs/goals` (`crates/nvs-ir/tests/refusals.rs:92`).
- `bun nv affected` takes the session base from `LOOPER_SESSION_BASE` (`cmd/affected.ts:54`). looper
  sets it for a hook, check or verify step (`crates/looper-engine/src/driver.rs:535` in looper's tree),
  but not in the agent's own environment, which gets only `LOOPER_WRITTEN` and `LOOPER_AGENT`
  (`crates/looper-agent/src/claude.rs:138-140`). So in a session's own shell, `bun nv affected` measures
  the uncommitted paths. A session commits only after it verifies, so nothing is missed in practice.
- `bun nv guard`'s `goal-grep` rule is deleted. A goal file now holds prose and the handoff as well as the
  checks, and the rule denied this session's own grep for the goal's headings. The `heavy-run` rule
  denies `looper check` without `--name` in place of `bun nv loop --goal-only|--run`.
- `bun nv owners` knows a goal by its file and counts the playbook's bullet files
  (`cmd/owners.ts:142`, `:226`), and `tools/nv/test/registers-reads.test.ts` watches
  `docs/agent/playbook/`. `bun nv audit checks` reads every goal file's checks (`cmd/audit.ts:99`).
- `accept/sweep.ts` and `bun nv plan` name `docs/goals/` in their messages.
- CI no longer runs `bun nv chain --check` or `bun nv playbook --check`.

**Two Stage 2 items moved to Stage 3, because each goes with what it serves.** `schema/index.ts:10-17`
still registers the `chain`, `goal`, `handoff` and playbook record types: unregistering them while
`data/` still holds their files makes `bun nv check` report each file as an orphan
(`tools/nv/lib/index.ts:299`). The playbook trailers stay in nv's form until
`tools/nv/lib/playbook-fragments.ts` is deleted, and its only users are `cmd/orient.ts`,
`cmd/playbook.ts`, `cmd/session.ts` and `driver/test/playbook.test.ts`.

**For the user: `.claude/settings.json` loses entries in Stage 3.** Once the seven loop commands are
deleted, its `bun nv` allow entries for `chain`, `goal`, `loop`, `loop-stats`, `orient`, `playbook` and
`session` name commands that no longer exist. The runbook's § 4 lists their lines. No session edits that
file, so removing them is the user's change.

### Next

**Stage 3: the deletes.** Files: `tools/nv/driver/`, `tools/nv/cmd/{loop,chain,goal,session,orient,loop-stats,playbook}.ts`,
`tools/nv/main.ts`, `tools/nv/schema/index.ts`, `tools/nv/lib/playbook-fragments.ts`, `tools/nv/lib/handoff.ts`,
`tools/nv/renderers/goal-plan.ts`, `tools/nv/lib/written.ts`.

- [ ] **The driver and the seven loop commands**, with their entries at `tools/nv/main.ts:34-74` and
  `tools/nv/driver/test/`.
  `lib/handoff.ts` goes with `cmd/session.ts`, its only user. The check `no-import-of-the-driver` holds it.
- [ ] **The record types and their data together**: `tools/nv/schema/index.ts:10-17` drops `chain`,
  `goal`/`sideGoal`, `handoff`/`sideHandoff` and `playbookSection`/`playbookBullet` in the same commit
  that deletes `data/chain.json`, `data/goals/` and `data/playbook/`. Their tests follow:
  `tools/nv/test/index.test.ts:60-71`, `records.test.ts:4-10` and `:91`, `store.test.ts:60-63`,
  `no-hand-build.test.ts:11`, `surfaces.test.ts:11`. `bun nv render` stops writing their JSON Schemas
  under `data/schema/`.
- [ ] **`renderers/goal-plan.ts`** and `docs/agent/goal-plan.md`, registered at `tools/nv/renderers/index.ts:15`;
  `tools/nv/test/render.test.ts:90-98` tests it.
- [ ] **The playbook trailers**, once `tools/nv/lib/playbook-fragments.ts:19` (`EXPIRY`) is deleted: each `[until: <kind> <arg>]`
  under `docs/agent/playbook/` becomes `` `until: <kind> <arg>` ``, and a `rule <topic>/<slug>` becomes
  `exists docs/rules/<topic>/<slug>.md`.
- [ ] **`tools/nv/lib/written.ts:10`** stops writing `NOVIS_LOOP_WRITES`, and the old driver's files under
  `.loop/` (`session-start.json`, `chain.json`, `run.json` and the rest the driver wrote) are deleted.
- [ ] **`docs/agent/goals/` and the prompts** (`docs/agent/looper-cutover.md:176`), once AGENTS.md
  holds their rules.

### Backlog

- Stage 4: the docs. The crate doc comments that still say "the goal records under `data/goals/`"
  (`crates/nvs-cli/tests/openapi.rs:394`, `crates/nvs-db/src/catalog.rs:2426`,
  `crates/nvs-stdlib/src/mail.rs:1148`, `crates/nvs-stdlib/src/storage.rs:597`,
  `crates/nvs-stdlib/tests/tier_boundary.rs:36`, `crates/nvs-types/tests/visibility.rs:14`,
  `crates/nvs-stdlib/tests/spec_registry_coverage.rs:39`) are rewritten with them.
