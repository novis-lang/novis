---
looper: 1
title: "the comparison is recorded on the tree with the lower request cost, and its page says what each figure measures"
summary: "Reached when benches/compare.json holds one run of every case and every feature row taken after request-fixed-cost and json-one-touch were reached, the website shows that run, the page says why Go is slow on the HTML page and what Novis does on every request that Bun and Go do not, and every figure in the page's prose is the record's."
context: [benches/compare/README.md, website/src/content/docs/in-depth/compare.mdx, website/src/lib/compare.ts]
stages:
  - id: 1
    title: the cases and the feature rows are recorded from a clean tree
    status: done
    context: [tools/nv/cmd/bench-compare.ts]
  - id: 2
    title: the page says what each figure measures, and its prose has the record's figures
    status: done
    context: [docs/agent/prose.md]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: done
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

- [x] `git worktree add --detach .agent-tmp/worktrees/compare HEAD`, then `bun install` in it.
- [x] In the worktree, `bun nv bench-compare --record benches/compare.json`, then
  `bun nv bench-compare --scenarios --record benches/compare.json`. Each is the item's own run.
- [x] Copy the worktree's `benches/compare.json` over the tree's, and bring the website's copy up to
  date with the command the subagent named.
- [x] Read the record's two `commit` values, and confirm with `git cat-file -e <commit>:<path>` that
  neither goal file exists at either. Write both commits into `### State`.
- [x] In the tree, `bun nv bench-compare --direct` once. Write the three peers' CPU per request into
  `### State`.
- [x] `git worktree remove .agent-tmp/worktrees/compare`, never with a forced form.

### 2. the page says what each figure measures, and its prose has the record's figures

- [x] `website/src/lib/compare.ts:59`: the `html-page` text gains the three sentences of
  § *The page's two additions*.
- [x] `website/src/content/docs/in-depth/compare.mdx:68`: the paragraph of § *The page's two
  additions*, under `<CaseCharts />`.
- [x] Every figure, ratio and ranking written as prose in `compare.mdx` and in `compare.ts` is compared
  with the record and corrected. A sentence that names which server is first in a case is corrected
  to the record's order.
- [x] `benches/compare/README.md:64`: the `html-page` paragraph gains one sentence with the measured
  cost of `html/template` and the reason the program keeps it.

### 3. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done, and the goal is reached. `looper verify` was green on its first run, with
nothing to fix. The goal-end gates are green: `bun nv verify --doc`, `bun nv owners --closes
compare-recorded` (no gap) and `bun nv ci --check`, whose advisories pass on every workspace.

- The standing decision to record from a detached worktree now lives in `benches/compare/README.md`
  § *One record*, under "A record is taken from a clean tree". The other decisions already had a
  home there: line 69 says why Go keeps `html/template`, and § *The direct measure* says `--direct`
  is never recorded.
- `RUSTSEC-2026-0206` and `RUSTSEC-2025-0141` stay in `deny.toml`. They match crates in
  `extensions/pdf` and `website/guides/highlight-extension`, which the root run does not see, so its
  warning is expected and the per-workspace runs need the entries.

`benches/compare.json` is one run at `068051b52c`, which the website shows. The Go note is on
`html-page` in `website/src/lib/compare.ts:56`, and the isolate paragraph is at `compare.mdx:70`.

The sweep's reds: `env-secret-proofs` is stale, because its newest `git log -S` hit is
json-one-touch's retire commit and it has no log. The nightly `supply chain` red (RUSTSEC-2026-0543,
`chrono` through `rxing`) is fixed on disk by `0fe2fdef25`, which postdates that run.

### Next

Nothing. The goal is reached, and the driver runs its acceptance checks and retires it. No test is
expected to go red.

### Backlog

