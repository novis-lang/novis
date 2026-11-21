---
looper: 1
title: "the comparison measures seven servers, and a routine recording does less work than the four-server one did"
summary: "Reached when bun nv bench-compare measures Novis, PHP, Bun, Go, Node.js, .NET and Python, a run plans its work before it starts, a routine seven-peer recording brings up fewer stacks than the four-peer one did, benches/compare.json holds every peer in every case and every row, and the comparison page shows all seven."
context: [benches/compare/README.md, tools/nv/cmd/bench-compare.ts, tools/nv/test/bench-compare.test.ts]
stages:
  - id: 1
    title: a run plans its work before it starts, and says where its time went
    status: done
    context: [tools/nv/cmd/bench-docker.ts, tools/nv/cmd/bench.ts]
  - id: 2
    title: a recording builds once, checks answers on its first rep, and reuses an unchanged peer's rows
    status: done
    context: [tools/nv/cmd/bench-docker.ts, website/src/lib/compare.ts]
  - id: 3
    title: Node.js is a peer
    status: done
    context: [benches/compare/compose.bun.yaml, benches/compare/bun/serve.ts, benches/compare/nginx/http.conf]
  - id: 4
    title: .NET is a peer
    status: done
    context: [benches/compare/compose.go.yaml, benches/compare/go/Dockerfile, benches/compare/nginx/http.conf]
  - id: 5
    title: Python is a peer
    status: done
    context: [benches/compare/compose.go.yaml, benches/compare/nginx/http.conf, docs/rules/tooling.md]
  - id: 6
    title: the seven-peer comparison is recorded, and its passes decide whether three reps are enough
    status: done
    context: [benches/compare.json]
  - id: 7
    title: the comparison page shows seven servers
    status: done
    context: [website/src/content/docs/in-depth/compare.mdx, website/src/lib/compare.ts, website/src/components/compare/FeatureTable.astro, website/src/components/compare/CaseCharts.astro, docs/agent/prose.md, docs/alternative-pages.md]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: plan
    stage: 1
    name: "a run's plan counts every stack, image build and fixed-load second, --plan starts no docker, and today's four-peer plan is pinned"
    run: [bun, test, tools/nv/test/bench-compare.test.ts, -t, "the plan"]
    timeout: 600
    tests:
      - "the plan counts every stack, image build and fixed-load second a run takes"
      - "the plan starts no docker"
      - "the plan of the four-peer recording is the baseline"
  - id: less-work
    stage: 2
    name: "a recording builds each image once, brings no stack up only to check answers, and reuses a row whose peer image, programs, tool and host are unchanged"
    run: [bun, test, tools/nv/test/bench-compare.test.ts, -t, "the work a recording does"]
    timeout: 600
    tests:
      - "the work a recording does: each image is built once and no stack is brought up with --build"
      - "the work a recording does: answers are checked on the first rep's stacks"
      - "the work a recording does: a row with an unchanged key reuses its recorded result"
      - "the work a recording does: a changed image, program, tool or host runs the row again"
      - "the work a recording does: the four-peer plan brings up fewer stacks than the baseline"
  - id: peers
    stage: 5
    name: "the peer table names Novis, PHP, Bun, Go, Node.js, .NET and Python, and every case and row has a program for every peer"
    run: [bun, test, tools/nv/test/bench-compare.test.ts, -t, "the peer table|every case has a program|every row has a program"]
    timeout: 600
    tests:
      - "names Novis, PHP, Bun, Go, Node.js, .NET and Python, in that order"
      - "every case has a program for every peer"
      - "every row has a program for every peer it does not give a fixed result"
  - id: recorded
    stage: 6
    name: "benches/compare.json holds every peer in every case and every row, the reps default matches what its passes show, and a routine seven-peer recording plans fewer stacks than the four-peer baseline"
    run: [bun, test, tools/nv/test/bench-compare.test.ts, -t, "the record"]
    timeout: 600
    tests:
      - "the record holds every peer in every case and every row"
      - "the record: three of five passes give the same figures and order within the bounds, or the default stays five"
      - "the routine seven-peer plan brings up fewer stacks than the four-peer baseline"
  - id: site
    stage: 7
    name: "the comparison page is reread and stamped"
    run: [bun, nv, site, --check, stale]
    timeout: 900
---

# the comparison measures seven servers, and a routine recording does less work than the four-server one did

## Why

The comparison page measures Novis against PHP, Bun and Go. A team that chooses a stack for a new web
application also weighs Node.js, .NET and Python, and the page says nothing about them. The planned
alternative pages (`docs/alternative-pages.md`) need a measured peer for each language they name.

Three more peers would make a recording almost twice as long. The record shows where the time goes:
each case pass sends 20,000 requests, and at the recorded rates all four peers' load over five reps is
about 160 seconds. Everything else is fixed work. That is one fresh stack per (case, rep, peer), a
second stack per (case, peer) only to check answers, two `docker compose up --build` calls per stack
(`tools/nv/cmd/bench-docker.ts:63`), and the feature rows, which run every peer again on every recording
although only Novis changes between two of them. So the cut is in the fixed work, and the figures
keep every rep, case and row.

## Standing decisions

**Python is measured, and no rule changes (the user's call to include it; the reading is the
authoring session's).** `rule:tooling/python-claims` permits "any measured figure, quoted with its
engine, mode and host". It requires the page to say that Novis has no REPL and no numeric or
machine-learning stack. The README's sentence that Python is absent because of that rule is wrong,
and stage 5 replaces it. "Faster than Python" and "replaces Python" stay forbidden on the page.

**The servers (the authoring session's calls).** Each is the deployment its own documentation shows,
tuned to its own advantage, with every setting in git, as README § *Why these four peers* requires.

| Peer | Server | Why |
|---|---|---|
| Node.js | `node:http`, with `node:cluster` starting one worker per cpu of the budget | Standard library only, and the same shape as Bun's one process per cpu |
| .NET | ASP.NET Core minimal API on Kestrel, a framework-dependent Release build, at its defaults (server GC, tiered PGO) | Kestrel ships with .NET itself, so it is standard library. JIT with PGO is the default deployment and has the higher throughput. Native AOT is not used |
| Python | An ASGI program with no framework, under `uvicorn` with `uvloop` and `httptools`, `--workers` equal to the budget's cpus | Python's standard library has no production server: the `http.server` documentation says so. `uvicorn` is the server FastAPI's documentation deploys with, and holds PHP-FPM's place for PHP. It is the one non-standard piece, and the README says so |

Each image is pinned to an exact tag: Node.js's current LTS, .NET's current LTS, and Python's current
stable release. A peer that cannot hold a case at the budget is measured as it is. Its result shows
the failure, and no setting is changed to hide it.

**How each new peer writes the cases (the authoring session's calls).** HTML escaping uses what the
peer's documentation names. For Python, that is `html.escape`. For .NET, it is
`HtmlEncoder.Create(UnicodeRanges.All)`, so `café` is not encoded. Node.js has no escaper in its
standard library, so its program has a five-character replace function, and the README says so. A
spelling of `'` or `"` the agreement mapping does not cover is added to `htmlEntities` only when it
is an entity for the same character. Any other difference is a disagreement, and the session fixes
the program. For JSON, .NET decodes into typed records with `System.Text.Json`, as Go does. Node.js and
Python test the decoded value, as PHP and Bun do.

**A row's result is read off what the peer did.** Only a row where the peer has no mechanism at all
takes a fixed result, with a link to the page in its own documentation (taint, secret, and session
or config where there is none). Node.js's `Worker` honours `resourceLimits`, unlike Bun's. So its
`memory` row may be `your-code`, and the run decides it.

**What the cut keeps (the authoring session's calls, for the run's length).** Every case and every
row stays, and the peers still interleave. A fresh stack per pass stays, because
`memory.peak` must be that pass's alone. One stack is up at a time. The cut is in three places:

1. **Each image is built once,** at the start of a run. `up` drops `--build`.
2. **The answers are checked on the first rep's stacks.** No stack is brought up only to check them.
   No figure is recorded until every peer agrees, so a disagreement still stops the run before any
   record is written.
3. **A feature row reuses a peer's recorded result when its key is unchanged.** The key is the peer's
   image id, the digest of the row's program folder, the digest of `bench-compare.ts` and
   `bench-docker.ts`, and the host the record names. The tool's digest also covers every tuning
   file in `TUNABLES` (stage 2's call): a changed nginx config or pool can move a result as much as a
   changed program, and the cost is only a row run again after a tuning change. Novis's image changes with every commit, so its
   rows always run. `--fresh` runs every row again. The record keeps a date and a commit for each
   result, and the page shows the oldest and the newest. The cases are never reused. A case's
   figures are relations between peers, measured in one interleaved run.

**Three reps are tested in this goal, not kept for later (the user's call).** Reps multiply every
case pass, so 3 instead of 5 removes 40% of the case stacks in every future recording. Stage 6 records
at 5 reps, with every pass kept, and that record is the test. For each of the ten ways to choose three
of the five passes, the median of those three is compared with the median of all five, for every
case and peer. 3 becomes the default when every one of those subsets meets all three bounds:

- requests per second within 3%
- peak memory within 5%
- p99 within 10%

The order of the peers by requests per second must also be the same in every case. That order is
what the page shows. Otherwise 5 stays, and the README records the worst deviation that decided it.
`cores` and `waiting` take their passes from `--reps` too, and the same test covers their figures.
The record stays the 5-rep one either way, because it is valid. A failed test is not the goal's red.
The goal is reached either way, with the decision and its numbers recorded.

If the routine seven-peer plan still brings up more stacks than the four-peer baseline after the
three cuts and the reps decision, the plan's check is the goal's red, and the handoff says by how
much.

**The plan is the count, and the clock is a note.** `planOf` is a pure function of `PEERS`, `CASES`,
`SCENARIOS`, the reps and the prior record. The run walks the same plan, so the count and the run
cannot drift. It counts stack bring-ups, image builds, fixed-load seconds and requests. The run also
times each phase and keeps the seconds in the record, so the next recording shows where time went.
Seconds never gate anything.

**A launcher whose worker exits ends the container (stage 3's call).** `node/serve.mjs` exits when any
cluster worker exits, as `bun/serve.ts` does. Node.js's documentation says a process must exit after
an uncaught exception, so the `crash` row's throw ends a worker. A primary that forks a replacement
would hide the crash from the restart count while the worker's other requests were lost, and the
row would read `built-in`. The container's restart policy is the supervisor, as for every peer. The
same holds for .NET and Python where a crash ends a process. Node.js's `memory` row is `your-code`
through `MEMORY_IN_CODE` (`Worker` `resourceLimits`), and its programs stay the plain ones its
documentation shows, as Bun's do.

**`--peer <id>,...` narrows a smoke run or a row to the named peers,** for checking a new peer
cheaply. `--record` refuses it, because a record is one run of every peer. One peer alone compares
its answers with nothing, so a new peer's smoke names a checked peer beside it (stage 5's call):
stages 3 and 4 smoked Node.js and .NET alone, and their agreement was first checked in stage 5.

**What Python's programs are (stage 5's calls).** CPython 3.15.2 on `python:3.15.2-slim-trixie`,
the standard build with the GIL. `python/Dockerfile` installs the wheels with `--only-binary` and
holds every `uvicorn` flag in its `ENTRYPOINT`: `uvloop`, `httptools`, `--lifespan off`,
`--no-access-log` (no line per request, as .NET), and `--timeout-keep-alive 65` (as Node.js). The
compose file and `--direct` pass only `--app-dir` and `--workers`. `PYTHONDONTWRITEBYTECODE=1`, so a
deploy part rewritten at the same size is never read from a stale `.pyc`. `escape` uses an f-string,
because the standard library has no HTML template. `parallel` uses a `ProcessPoolExecutor`, because
threads run one at a time. `loop` and `memory` have no `your-code` entry: a Python thread cannot be
stopped, and a child process per request is a second program, as for Go and .NET. `json-api` turns
`NaN` and `Infinity` away, which `json.loads` reads and JSON does not have.

**What it costs.** No request path changes, and no memory is spent. The nightly `flood` job in
`.github/workflows/ci.yml` builds three more images, with .NET's SDK the largest. That job gates
nothing.

**Lanes.** Stages 3, 4 and 5 share `PEERS`, the `Setting` tables and the test file, so they run in
order. Each peer's programs are disjoint files, so within a stage they go to writing subagents.
Stage 7 touches only the website and `docs/alternative-pages.md`, and needs stage 6's record.

**The charts colour by place, not by peer (stage 7's call).** Every bar in place 1 takes the accent
colour, Novis's bars have a border, and the deploy boxes take their group's colour. No chart has a
colour per peer, so no new peer needs one: seven colours would carry no meaning the place does not.
The feature table scrolls inside its own box with the feature column sticky, so the page never
scrolls sideways at phone width. The page's prose reads each new peer's result off the record.

## Stages

### 1. a run plans its work before it starts, and says where its time went

**Split:** none, one session.

- [x] `tools/nv/cmd/bench-compare.ts:2423` `main`: a pure `planOf(peers, cases, scenarios, reps, prior)`
  returns the ordered passes, and each has its stack, its image builds and its fixed-load seconds.
  The case loop at `tools/nv/cmd/bench-compare.ts:2461` and `scenarios` at
  `tools/nv/cmd/bench-compare.ts:2341` walk it.
- [x] `tools/nv/cmd/bench-compare.ts:2242` parse: `--plan` prints the totals per phase and exits
  before `requireDocker`.
- [x] `tools/nv/cmd/bench-compare.ts:1740` `onePass` and `tools/nv/cmd/bench-compare.ts:1830`
  `scenarioPass`: time each phase (up app, up nginx, answers, load, down). `buildRecord` at
  `tools/nv/cmd/bench-compare.ts:1576` keeps the seconds per case and row under `seconds`, and the
  `passes` of every rep beside the median.
- [x] `tools/nv/test/bench-compare.test.ts:100`: the three `plan` tests. The baseline test pins
  today's four-peer counts as literals.
- [x] `benches/compare/README.md:257` § *One record*: a paragraph on the plan and the `seconds`.

### 2. a recording builds once, checks answers on its first rep, and reuses an unchanged peer's rows

**Split:** none, one session.

- [x] `tools/nv/cmd/bench-docker.ts:63` `up`: no `--build`. A `build` step runs once per image at the
  start of a run, and `bench-proxied` keeps its own behaviour.
- [x] `tools/nv/cmd/bench-compare.ts:2446`: remove the separate agreement pass, and check agreement on
  rep 1's answers before rep 2 starts.
- [x] `tools/nv/cmd/bench-compare.ts:2341` `scenarios` and `tools/nv/cmd/bench-compare.ts:1622`
  `buildScenarios`: the reuse key, `--fresh`, a date and commit per result. `tools/nv/cmd/bench-compare.ts:1669`
  `digests` gives the program-folder digest.
- [x] `tools/nv/cmd/bench-compare.ts:2242` parse: `--peer <id>`, refused with `--record`.
- [x] `tools/nv/test/bench-compare.test.ts:249`: the five `the work a recording does` tests.
- [x] `benches/compare/README.md:239` § *Agreement before timing* and `benches/compare/README.md:257`
  § *One record*: the first-rep check and the reuse key.

### 3. Node.js is a peer

**Split:** a writing subagent writes `benches/compare/cases/*/server.mjs` and every
`benches/compare/scenarios/*/server.mjs`, modelled on Bun's `server.ts` files. The session writes the
compose file, `PEERS` and the settings, then runs the smoke.

- [x] `benches/compare/compose.node.yaml:1` and `benches/compare/node/serve.mjs:1`: the stack, the image
  pin and the cluster launcher. It uses `nginx/http.conf`.
- [x] `tools/nv/cmd/bench-compare.ts:126` `PEERS`: the `node` entry. `tools/nv/cmd/bench-compare.ts:1932`
  `DIRECT_RUNS`: its direct run. `tools/nv/cmd/bench-compare.ts:506`, `:636`, `:702`, `:817`, `:929`,
  `:1002`: each `Setting` and deploy table that names a peer.
- [x] The programs for the 4 cases and every row with per-peer programs.
- [x] Requirement: `bun nv bench-compare --smoke --peer node` and `bun nv bench-compare --scenarios
  --smoke --peer node` are green. Run them in the detached worktree that README § *One record* names.
- [x] `tools/nv/test/bench-compare.test.ts:101`: the peer-table test names Node.js.

### 4. .NET is a peer

**Split:** as stage 3. A writing subagent writes `Program.cs` for the 4 cases and every row. The
session writes the compose file, the Dockerfile, `PEERS` and the settings.

- [x] `benches/compare/compose.dotnet.yaml:1` and `benches/compare/dotnet/Dockerfile:1`: the SDK build
  stage and the ASP.NET runtime stage, with every case and row built into the image as Go's are.
- [x] `tools/nv/cmd/bench-compare.ts:126` `PEERS`, `tools/nv/cmd/bench-compare.ts:1932` `DIRECT_RUNS`,
  and the `Setting` and deploy tables.
- [x] The programs for the 4 cases and every row with per-peer programs.
- [x] Requirement: both smoke runs with `--peer dotnet` are green.
- [x] `tools/nv/test/bench-compare.test.ts:101`: the peer-table test names .NET.

### 5. Python is a peer

**Split:** as stage 3. A writing subagent writes `app.py` for the 4 cases and every row.

- [x] `benches/compare/compose.python.yaml:1` and `benches/compare/python/requirements.txt:1`: the image
  pin, and `uvicorn`, `uvloop` and `httptools` at exact versions.
- [x] `tools/nv/cmd/bench-compare.ts:126` `PEERS`, `tools/nv/cmd/bench-compare.ts:1932` `DIRECT_RUNS`,
  and the `Setting` and deploy tables.
- [x] The programs for the 4 cases and every row with per-peer programs.
- [x] Requirement: both smoke runs with `--peer python` are green.
- [x] `tools/nv/test/bench-compare.test.ts:101`: the peer-table test names all seven, in that order.
- [x] `benches/compare/README.md:14`: the section is § *Why these peers*, with no count to go stale. It
  gives each new peer's server and why, says `uvicorn` is the one non-standard piece, and replaces
  the Python sentence.

### 6. the seven-peer comparison is recorded, and its passes decide whether three reps are enough

**Split:** none. The recording runs in the background, and the session ends its turn to wait.

- [x] Requirement: in a detached worktree at `HEAD`, run `bun nv bench-compare --record
  benches/compare.json`, then `bun nv bench-compare --scenarios --fresh --record benches/compare.json`,
  and copy the record back (README § *One record*).
- [x] `tools/nv/cmd/bench-compare.ts:1899` `repsHold`: a pure `repsHold(passes)` runs the three-of-five
  test from § *Standing decisions* over the record's kept passes. It returns the worst deviation for
  each figure, and whether the order changed. `bun nv bench-compare --reps-check benches/compare.json`
  prints it with no docker.
- [x] Requirement: run `--reps-check` on the new record. When it holds, the default at
  `tools/nv/cmd/bench-compare.ts:3097` becomes 3. Either way, `benches/compare/README.md:257`
  § *One record* states the reps, the worst deviation and the date of the record that decided them.
- [x] `tools/nv/test/bench-compare.test.ts:1`: the three `the record` tests. The plan test uses the
  record as the prior, with Novis's key changed, at the reps default the check decided.

### 7. the comparison page shows seven servers

**Split:** none, one session.

- [x] `website/src/lib/compare.ts:145` `PEER_GROUP`: a group for each new peer, from its measured deploy
  rows. Fix every text that says "all four" at `website/src/lib/compare.ts:128` and
  `website/src/lib/compare.ts:259`.
- [x] `website/src/components/compare/FeatureTable.astro:17`: seven columns work at phone width with no
  page scroll. Each chart in `website/src/components/compare/` gets a colour for each new peer.
- [x] `website/src/content/docs/in-depth/compare.mdx:2`: the title, description and every "four"
  name seven servers. The page says that Novis has no REPL and no numeric or machine-learning stack,
  as `rule:tooling/python-claims` requires. It shows the date range of the reused rows. Reread it
  against `docs/agent/prose.md` and stamp it with `bun nv site --stamp`.
- [x] `docs/alternative-pages.md:1` § *The peers*: say the three peers are measured, and point to the
  README section that is now their home.
- [x] The goal moves no feature, so it writes no `website/src/data/changes.json` entry.

### 8. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 7 are done. Stage 7: `website/src/lib/compare.ts` groups the three new peers as
`build` and exports `rowDates`, which `Machine.astro` shows as the feature table's date range
(2026-11-20 to 2026-11-21). The feature table scrolls in its own box. `compare.mdx` names seven
servers, says Novis has no REPL and no numeric or machine-learning stack, and reads each new peer's
results off the record. `bun nv render` refreshed `website/src/data/compare.json`, which still held
the four-peer record of 2026-11-18. The page and `in-depth/index.mdx` passed a newcomer review and are
stamped, and so are their questions. `site --check stale changes review` is green.

`benches/compare.json` is the seven-peer record: cases at `0a0aa84bbd`
(2026-11-20), rows at `d676c2f8fb` (2026-11-21), every peer in every case and all 18 rows. The rows
took three background calls: a stopped run resumed without `--fresh` and kept its finished rows.

`--reps-check` fails every bound, so `DEFAULT_REPS` stays 5. Worst: requests/sec 16.52% (`waiting`
at 1 cpu, .NET), peak memory 18.17% (`hello`, Bun), p99 63.03% (`hello`, Python), and 5 subsets
reorder `hello` or `wait`. README § *Five reps, because three did not give the same figures* records
it. A routine seven-peer recording plans 140 case stacks plus 39 row stacks (85 row results kept),
179 in all, against the four-peer baseline of 247: `--scenarios --plan --record
benches/compare.json`. `--plan` reads no prior without `--record`, so it then counts every row.

The `the record` tests and the record-shape test, which listed four peers, are green by name. The
nightly failures ran on `5d053fc6e6`, origin/main. Local `e5a100b18f` and `438e9c44af` fix them, and
reach CI at the next full gate's push.

### Next

Stage 8, the gate: the check the pack names (`looper verify` at a full gate, else a build and the
`bench-compare` tests by name), then `bun nv owners --closes compare-seven-peers`, and move each
standing decision only this file holds into `benches/compare/README.md` before the file is deleted.
Expect no test red.

### Backlog

- The `parallel` row records Novis as **One setting** (`[capabilities.script] spawn`,
  `tools/nv/cmd/bench-compare.ts:1366`), while `compare.mdx` § *One request in four parts* says Novis
  needs no setting for a script in its own app folder. The row's script or the sentence is wrong.
