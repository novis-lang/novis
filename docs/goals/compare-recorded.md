---
looper: 1
title: "the comparison is recorded on the tree with the lower request cost, and its page says what each figure measures"
summary: "Reached when benches/compare.json holds one run of every case and every feature row taken after request-fixed-cost and json-one-touch were reached, the website shows that run, the page says why Go is slow on the HTML page and what Novis does on every request that Bun and Go do not, and every figure in the page's prose is the record's."
context: [benches/compare/README.md, website/src/content/docs/in-depth/compare.mdx, website/src/lib/compare.ts]
stages:
  - id: 1
    title: the cases and the feature rows are recorded from a clean tree
    status: open
    context: [tools/nv/cmd/bench-compare.ts]
  - id: 2
    title: the page says what each figure measures, and its prose has the record's figures
    status: open
    context: [docs/agent/prose.md]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: record-reads
    stage: 1
    name: "the record parses as the tool's tests read it, and every stack still serves every case"
    run: [bun, nv, bench-compare, --smoke]
    timeout: 1800
  - id: page-builds
    stage: 2
    name: "the website builds, and the comparison page's figures are the record's"
    run: [bun, nv, ci, --check]
    timeout: 1800
---

# the comparison is recorded on the tree with the lower request cost, and its page says what each figure measures

## Why

The comparison page ranks Novis third on `hello` and on `json-api`. `request-fixed-cost` and
`json-one-touch` lower what Novis spends on those requests, and the page shows nothing of it until the
comparison is recorded again.

The page also leaves two things unsaid, and a reader fills each gap with a wrong guess:

- **Go answers the HTML page at a fifth of Bun's rate.** That looks like a mistake in the benchmark.
  It is not: measured alone on 2026-11-17, Go's handler takes about 290 µs and 2,394 allocations per
  page with `html/template`, and about 60 µs written by hand with `html.EscapeString`.
  `html/template` is the package Go's documentation names for HTML, and the comparison writes each
  peer the way its documentation shows (`benches/compare/README.md:98`).
- **Novis does work on every request that Bun and Go do not do.** It runs each request in its own
  isolate, under a CPU limit and a memory limit, and counts the memory the request allocates. The page
  shows the result of that in its `loop`, `memory` and `flood` rows, and never says that the same work
  is in every figure of the speed charts.

## Standing decisions

**One recording, taken the way the README says** (`benches/compare/README.md:295`): the cases first,
then the feature rows, from a detached worktree at `HEAD` under `.agent-tmp/worktrees/compare`, so
another session's half-written edit is never in the image. The loops keep running while it records.
The page's figures are relations, and the peers take turns within a run.

**No program of any peer changes.** The four programs of each case stay as they are. Go keeps
`html/template`. A faster Go program would be the comparison choosing how Go is written.

**The page's two additions (my call), written under AGENTS.md § *Text an end user reads*.** The
wording below is the wording to use, changed only where the record's figures require it.

- The `html-page` case's text in `website/src/lib/compare.ts:59` gains: "Go builds this page with
  `html/template`, the package its documentation names for HTML. `html/template` looks up the type of
  each value while it builds the page. That takes more time for each page than the other servers
  need."
- `website/src/content/docs/in-depth/compare.mdx` gains one paragraph under the case charts: "Novis
  runs every request in its own isolate, with a CPU limit and a memory limit. Bun and Go run all
  requests in one process and have no limit for one request. The rows `loop`, `memory` and `flood`
  below show what that changes."

**No chart of the direct measure goes on the page.** `bun nv bench-compare --direct` is a tool for the
people who work on Novis. The page shows the deployment a reader has, behind a proxy with TLS.

**Limits: what stops this goal.** At most two full recordings. A row that fails is reproduced alone
with `bun nv bench-compare --scenarios --row <id>` and fixed at its cause before the second recording.
A figure that did not improve is published as it is: no result holds this goal open, and nothing in
the programs, the settings or the page's ranking changes to improve one.

**Lanes.** One lane. Stage 2 reads the record stage 1 wrote.

## Stages

### 1. the cases and the feature rows are recorded from a clean tree

**Gate:** `docs/goals/request-fixed-cost.md` and `docs/goals/json-one-touch.md` do not exist.

**Split:** an `Explore` subagent first reports how `website/src/data/compare.json` is made from
`benches/compare.json`, as the command and its `path:line`.

- [ ] `git worktree add --detach .agent-tmp/worktrees/compare HEAD`, then `bun install` in it.
- [ ] In the worktree, `bun nv bench-compare --record benches/compare.json`, then
  `bun nv bench-compare --scenarios --record benches/compare.json`. Each is the item's own run.
- [ ] Copy the worktree's `benches/compare.json` over the tree's, and bring the website's copy up to
  date with the command the subagent named.
- [ ] Read the record's two `commit` values, and confirm with `git cat-file -e <commit>:<path>` that
  neither goal file exists at either. Write both commits into `### State`.
- [ ] In the tree, `bun nv bench-compare --direct` once. Write the three peers' CPU per request into
  `### State`.
- [ ] `git worktree remove .agent-tmp/worktrees/compare`, never with a forced form.

### 2. the page says what each figure measures, and its prose has the record's figures

- [ ] `website/src/lib/compare.ts:59`: the `html-page` text gains the three sentences of
  § *The page's two additions*.
- [ ] `website/src/content/docs/in-depth/compare.mdx:68`: the paragraph of § *The page's two
  additions*, under `<CaseCharts />`.
- [ ] Every figure, ratio and ranking written as prose in `compare.mdx` and in `compare.ts` is compared
  with the record and corrected. A sentence that names which server is first in a case is corrected
  to the record's order.
- [ ] `benches/compare/README.md:64`: the `html-page` paragraph gains one sentence with the measured
  cost of `html/template` and the reason the program keeps it.

### 3. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written on 2026-11-17 from a conversation with the person. Their calls: Go's low figure on the HTML
page looked like a benchmark bug to them, and nothing else is given up to improve these cases. The
wording of the page's two additions, and every other choice in § *Standing decisions*, is the
authoring session's call. Nothing is recorded yet.

### Next

Stage 1, once its gate holds.

### Backlog

