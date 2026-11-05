---
looper: 1
title: The comparison is recorded again on the faster request path
summary: "Reached when benches/compare.json's cases and feature rows were both recorded on a tree that holds ceiling-wake, json-one-pass and strings-in-place, and the comparison page shows that record with its prose figures matching it."
context: [benches/compare/README.md, tools/nv/cmd/bench-compare.ts, website/src/content/docs/in-depth/compare.mdx]
stages:
  - id: 1
    title: "the cases and the feature rows are recorded on the new tree, and the page matches the record"
    status: open
    context: [tools/nv/renderers/website-compare.ts, website/src/lib/compare.ts]
  - id: 2
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: record-is-current
    stage: 1
    name: both halves of the comparison record were taken on a tree that holds the three request-path goals
    run: [bun, -e, "const r=require('./benches/compare.json');const x=require('child_process').execFileSync;for(const c of [r.commit,r.scenarios&&r.scenarios.commit])for(const f of ['crates/nvs-host/src/watchdog.rs','crates/nvs-stdlib/src/json.rs','crates/nvs-runtime/src/string.rs']){const h=x('git',['log','-1','--format=%H','--',f]).toString().trim();x('git',['merge-base','--is-ancestor',h,c])}"]
    timeout: 120
---

# The comparison is recorded again on the faster request path

## Why

`benches/compare.json` was recorded on 2026-10-23 at `4f01826ad`, and the website's comparison page
shows it. `ceiling-wake`, `json-one-pass`, `json-any-size` and `strings-in-place` change the cost of
every served request, of JSON reading and writing at every size, and of html templates. The user
decided on 2026-11-05 that the comparison is recorded again once all four have landed, so the page shows what Novis does now.

The `record-is-current` check holds the goal open until both halves of the record (the cases and the
feature rows) name a commit that contains the newest change to `watchdog.rs`, `json.rs` and `string.rs`.

## Standing decisions

1. **The gate:** `docs/goals/ceiling-wake.md`, `docs/goals/json-one-pass.md`,
   `docs/goals/json-any-size.md` and `docs/goals/strings-in-place.md` are all deleted, which is how a goal is retired. Stage 1 starts
   only then. If one of them is still on the chain, the session writes `CONTINUE waiting on <slug>`
   and stops.
2. **Record from a detached worktree.** `bench-compare` builds the Novis image from the tree it runs in,
   so other sessions' half-written edits in the root would break the build. Make
   `.agent-tmp/worktrees/compare-record` with `git worktree add --detach` at `HEAD`, run `bun install`
   there, record there, and copy `benches/compare.json` back to the root. Remove the worktree with
   `git worktree remove` once the record is copied, never with `--force`.
3. **Record with the loops running.** The user decided that loops are not stopped for a recording. The
   record's medians of interleaved runs are what keeps it fair, and the commit message says that other
   work was running.
4. **Two runs, both background.** First `bun nv bench-compare --record benches/compare.json` for the
   cases, then `bun nv bench-compare --scenarios --record benches/compare.json` for the feature rows.
   Each runs with `run_in_background` and a timeout of 7200000, and the session ends its turn to wait.
   A row that fails is reproduced with `--scenarios --row <id>` without `--record`, fixed at its cause,
   and then all rows are recorded again.
5. **Docker Desktop.** If Docker is not running, start Docker Desktop with PowerShell `Start-Process`.
   A stale E: mount (`mkdir /run/desktop/mnt/host/e: file exists`) is fixed by quitting Docker Desktop,
   `wsl --shutdown`, and starting it again. Set `MSYS_NO_PATHCONV=1` for a `docker run -v`. Only an
   update that needs Windows' administrator prompt is `BLOCKED`.
6. **The page follows the record.** `bun nv render --website` copies the record into
   `website/src/data/compare.json`. Every figure written in the page's prose that the record measures
   is checked against the new record and corrected. The prose follows AGENTS.md § *Text an end user
   reads*. A result worse for Novis than before is shown as measured, never adjusted.
7. **Lanes:** one.

## Stages

### 1. the cases and the feature rows are recorded on the new tree, and the page matches the record

**Does:** takes the record, puts it on the page, and makes the page's text agree with it.

**Split:** a `reader` subagent compares every figure in the page's prose with the new record and
returns a list of `line: old → new`.

- [ ] Make the worktree and run the case record in the background, as decisions 2 and 4 say.
  `tools/nv/cmd/bench-compare.ts:1`
- [ ] Run the feature-row record in the background in the same worktree. `tools/nv/cmd/bench-compare.ts:1`
- [ ] Copy the record back, remove the worktree, and run `bun nv render --website`.
  `tools/nv/renderers/website-compare.ts:1`
- [ ] Correct each prose figure the record changed, such as the deploy rows' shares of mixed pages.
  `website/src/content/docs/in-depth/compare.mdx:153`
- [ ] Commit the record and the page. The message lists requests per second per case and peer, before
  (`4f01826ad`) and after.

### 2. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Waiting on `ceiling-wake`, `json-one-pass`, `json-any-size` and `strings-in-place`. Nothing is
recorded yet.

### Next

**Stage 1**, gated on decision 1: the three goal files are deleted.

- [ ] The case record, in the background. `tools/nv/cmd/bench-compare.ts:1`
- [ ] The feature-row record, in the background. `tools/nv/cmd/bench-compare.ts:1`
- [ ] Copy back, remove the worktree, render. `tools/nv/renderers/website-compare.ts:1`
- [ ] Correct the prose figures. `website/src/content/docs/in-depth/compare.mdx:153`
- [ ] Commit with the before and after figures. `benches/compare/README.md:1`

### Backlog
