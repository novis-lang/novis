---
looper: 1
title: "Novis, PHP, Bun and Go compared end to end, measured by one command, shown on the website"
summary: "When this goal is green, `bun nv bench-compare` brings up four stacks one at a time, each behind the same nginx terminating TLS, on the same CPU budget: `nvs serve`, PHP-FPM, Bun and Go."
context: [tools/nv/cmd/bench-proxied.ts, benches/proxied/README.md, benches/proxied/compose.nvs.yaml, benches/proxied/nginx/nvs.conf]
stages:
  - id: 2
    title: four peers behind one TLS proxy
    status: done
  - id: 3
    title: the four cases
    status: done
  - id: 4
    title: the Novis fact the table needs and nothing tests
    status: done
    context: [crates/nvs-cli/tests/live_edit.rs, crates/nvs-server/src/serve.rs]
    extra:
      context:
        rules: [security/isolate-shares-nothing]
  - id: 5
    title: the feature scenarios
    status: done
    extra:
      context:
        rules: [config/an-edit-reaches-the-next-request-without-a-restart, config/a-request-keeps-the-unit-it-resolved, core-classes/html-auto-escape]
  - id: 6
    title: the page
    status: done
    context: [tools/nv/cmd/render.ts, website/astro.config.ts]
  - id: 7
    title: the first recorded run
    status: done
checks:
  - id: bench-compare-unit
    stage: 2
    name: "bench-compare: the peer table, the record shape and the agreement mapping"
    run: [bun, test, tools/nv/test/bench-compare.test.ts]
    cwd: .
  - id: four-peers-agree
    stage: 3
    name: "bench-compare --smoke: all four peers answer every case alike behind TLS"
    run: [bun, nv, bench-compare, --smoke]
    cwd: .
    stdout_contains: ["hello: 4 peers agree", "json-api: 4 peers agree", "html-page: 4 peers agree", "wait: 4 peers agree"]
  - id: serve-isolation
    stage: 4
    name: nvs-cli (a static set by one request is not seen by the next under nvs serve)
    run: [cargo, test, --test, serve_isolation]
    tests: [a_static_set_by_one_request_is_not_seen_by_the_next]
  - id: every-scenario-runs
    stage: 5
    name: "bench-compare --scenarios --smoke: every feature row runs against every peer"
    run: [bun, nv, bench-compare, --scenarios, --smoke]
    cwd: .
    stdout_contains: ["cores:", "isolation:", "runaway:", "crash:", "escape:", "taint:", "secret:", "csrf:", "headers:", "waiting:", "parallel:", "live-edit:", "deploy:", "config:", "upgrade:"]
  - id: page-exists
    stage: 6
    file: website/src/content/docs/in-depth/compare.mdx
    min_bytes: 1000
  - id: recorded-run-on-the-website
    stage: 7
    output_contains: ["\"nvs\"", "\"php\"", "\"bun\"", "\"go\"", "\"host\"", "\"scenarios\""]
    file: website/src/data/compare.json
extra:
  files: []
  env: {}
  context:
    adrs: []
    milestones: []
    plan: []
    playbook: []
    rules: [http-server/two-deployments-and-nothing-a-proxy-owns, testing/userland-benchmarks, programs/three-claims]
    shapes: [A commit message]
---

# Novis, PHP, Bun and Go compared end to end, measured by one command, shown on the website

## Why

### The target

When this goal is green, `bun nv bench-compare` brings up four stacks one at a time, each behind the
same nginx terminating TLS, on the same CPU budget: `nvs serve`, PHP-FPM, Bun and Go. It runs four
cases against each, and checks that the four peers answer each case the same way before it times anything.
It then runs a fixed list of feature scenarios against each stack and records what each one did. Both
results go into one record, `benches/compare.json`, that names the machine they were taken on.
`bun nv render --website` copies that record to the website, and a page under *In depth* shows the
numbers as relations between the peers and the features as a table. Every cell on that page is
generated from a run and links to the script that produced it. Nothing on it is typed by hand.

### Why a side goal

The user asked for this on 2026-10-09, run by hand and off the chain. Nothing on the chain waits for it,
and it shares no file set with the live goal: `tools/nv`'s bench commands, `benches/`, one new test in
`nvs-cli` and the website.

### What is on disk today, measured

- **`bun nv bench-proxied`** (`tools/nv/cmd/bench-proxied.ts`, `benches/proxied/`) already runs
  `oha` → nginx → php-fpm against `oha` → nginx → `nvs serve` in Docker. It has the CPU budget as a
  compose key (`--arm fair` one CPU, `--arm deployed` four), `oha` pinned by digest, a static FPM pool
  with opcache and JIT, `config_digests` in every row, and a body-equality refusal (`:513`).
  `benches/proxied/README.md` is the home of every decision behind it, and this goal keeps them.
- **What it lacks:** nginx only listens on port 80, no TLS. No memory is measured anywhere in the leg;
  the only peak-memory helper is `peakMemory()` in `tools/nv/cmd/bench.ts:1114`. One case (`hello`),
  one concurrency, no sweep. The two peers are hard-coded: the ratios are fixed Novis/PHP pairs
  (`bench-proxied.ts:533-534`) and the `images` map is static.
- **The Novis facts the feature table rests on**, each checked on 2026-10-09:
  - one worker per CPU, `crates/nvs-cli/src/serve.rs:714`;
  - a new `Ctx` per request, `crates/nvs-server/src/serve.rs:2690`, but **no test sets a static in one
    request and reads it from the next under `nvs serve`** (stage 4 writes it);
  - per-request `memory`, `cpu_time` and `wall_time` exist and **default to no cap**,
    `crates/nvs-config/src/default.toml:69-71`;
  - a panic owned by a request leaves the worker running, `benches/abi-probe/tests/invariants.rs:285`;
  - `echo` accepts only markup and html templates escape their holes, `crates/nvs-codegen/src/emit.rs:585`,
    and the escaping is **not contextual**, `docs/rules/core-classes/html-template.md:52-55`;
  - an edit reaches the next request in production, `crates/nvs-cli/tests/live_edit.rs:400`; a
    copy that pauses less than `settle` compiles once from the finished tree, `:931`; a link switch never
    builds from both releases, `:1036`;
  - config is re-read when `nvs.toml` changes, checked every two seconds, `crates/nvs-cli/src/reload.rs:557`;
    `listen`, `socket_mode` and `workers` still need a restart;
  - a request's `sleep` parks its task and frees the core, `crates/nvs-host/src/timer.rs:282-288`;
  - `Core\Task::map` is concurrent on the request's own core, **not parallel**
    (`docs/rules/concurrency/one-scheduler.md:5`). Work on another core is `spawn script … on: "worker"`;
  - CSRF on by default covers `#[Route]` rows only, `crates/nvs-server/src/route.rs:113-122`;
  - secure headers by default, `crates/nvs-server/src/secure.rs:128`; `max_in_flight` answers 503,
    `crates/nvs-server/src/admit.rs:239`;
  - upgrading the `nvs` binary needs a restart: there is no socket handoff.
- **The website** is Astro with Starlight. Data is JSON under `website/src/data/`, written by
  `bun nv render --website` and read through `website/src/lib/*.ts`. No page shows a benchmark today.

### Stage 1 — the floor

Main's carried floor, which a side run is always checked against. Never traded.

## Standing decisions

These are the user's calls, made on 2026-10-09, unless marked as mine. No session re-decides one.

- **The peers are Novis, PHP, Bun and Go.** Not Node.js: it is the same language as Bun. Not Python:
  the tooling chapter's rule on Python claims forbids "faster than Python" in any document.
- **The comparison page may name PHP, Bun and Go wherever it needs to.** AGENTS.md § *Text an end
  user reads* states this exception.
- **No Docker-only runner.** `bun nv bench-compare` is the one way to run it; Docker and `bun` are what
  it needs.
- **A side goal**, run by hand in its own worktree.
- **End to end, behind a TLS proxy, with no third service.** Only what the language does itself: input,
  processing, output, timers.
- **The numbers are relations.** The page says the absolute figures depend on the machine.
- **Mine, 2026-10-09:** the four cases, their concurrencies and the metrics; the fifteen scenario rows
  and the four result values; the rows where Novis does not win (`isolation` and `runaway` are also
  PHP's strengths, `escape` is context-aware in Go and not in Novis, `parallel` is a tie with Bun
  and Go, `upgrade` needs a restart) stay on the page.
- **Mine, 2026-10-09:** `bench-compare` is a new command beside `bench-proxied`, sharing its helpers;
  `benches/compare.json` is a separate artifact from `benches/serve-proxied.json`, and no arithmetic
  joins them.
- **ADR slots: none.** Decision records are retired. A decision that outlives the goal goes into
  `benches/compare/README.md`, which is the home of why the comparison is built the way it is.
- **The tradeoffs, stated once.** The language and the runtime do not change, except stage 4's test and
  any security fix it forces. Contributors gain one command and four small programs per case to keep
  correct. The run takes minutes and needs Docker, so it is not part of `nv verify` or CI.

## Stages

### 2. four peers behind one TLS proxy

**Does:** Adds `bun nv bench-compare`, which brings up Novis, PHP-FPM, Bun and Go one at a time behind the same TLS-terminating nginx on the same CPU budget, and records requests per second, latency and memory for each.

File set: `tools/nv/cmd/bench-compare.ts` (new), the compose, Docker and `oha` helpers moved out of
`tools/nv/cmd/bench-proxied.ts` into a module both import, `benches/compare/` (new), its `README.md`,
`tools/nv/test/bench-compare.test.ts` (new).

- **Peers are data**: one entry per peer naming its compose file, its nginx upstream, its image pin and
  the settings written into every row. Adding a fifth peer is a new entry and a compose file, never a
  new code path. `bench-proxied` keeps working and keeps writing `benches/serve-proxied.json`, which
  stays a separate artifact (`benches/proxied/README.md` § *Two artifacts*).
- **One stack up at a time**, as `bench-proxied` does, with nginx and `oha` given the same CPUs in
  every stack. The `fair` arm gives each backend one CPU; the `deployed` arm gives it four.
- **TLS 1.3 at nginx**, HTTP/1.1 with keep-alive to the backend. The certificate is self-signed and
  generated into a git-ignored folder at the first run; `oha` runs with `--insecure`.
- **Each peer uses its standard library only and is tuned to its own advantage**, every setting in git:
  PHP 8.5 FPM with a static pool sized as `bench-proxied` sizes it, opcache and JIT; Bun's `Bun.serve`
  run as one process per CPU of the budget with `reusePort`; Go's `net/http` with `GOMAXPROCS` set to
  the budget; `nvs serve` with its defaults. Images pinned to a patch version, `oha` by digest.
- **Memory** is the backend container's cgroup `memory.peak`, read from a fresh container per arm,
  plus its `memory.current` at idle before the first request.
- **A smoke mode**, `--smoke`, runs every peer and case with a few hundred requests, checks the answers
  agree, and prints one line per case: `<case>: 4 peers agree`. It records nothing.

### 3. the four cases

**Does:** Writes hello, json-api, html-page and wait for all four peers, checked to answer alike before any timing.

File set: `benches/compare/cases/<case>/` with one file per peer (`.nvs`, `.php`, `.ts`, `.go`), the
fixtures they read, `tools/nv/cmd/bench-compare.ts`.

- **hello**: `GET /` returns `hello, world\n`. The cost of the server alone.
- **json-api**: `POST /orders` with a JSON order of about 2 KB (customer, 20 line items). Check the
  types and ranges, compute line totals, tax and a grand total in **integer cents**, sort the items by
  total, and return compact JSON in a fixed key order. A body that fails the checks returns 422 with a
  fixed error body. Integer cents and a fixed key order are what make the four answers byte-equal.
- **html-page**: `GET /products?q=<term>` renders a 100-row table from data built in memory and shows
  the term in the heading, escaped. The term in the run contains `<`, `>`, `&`, `"` and `'`. Each peer
  escapes the way its own documentation says to. The answers are compared after mapping `&#039;`,
  `&#x27;` and `&apos;` to `&#39;`, and `&#34;` to `&quot;`, and nothing else.
- **wait**: each request waits 50 ms on a timer, then answers. Run at 1000 concurrent connections.
  This is the "memory for 1000 requests in flight" figure. The FPM pool is sized for it, as a real one
  would be.
- **Agreement before timing**: same status, same `Content-Type`, same body after the case's mapping,
  for a fixed set of inputs including the 422 one. A disagreement stops the run and names the peer.
- **Concurrency**: 64 for hello, json-api and html-page, 1000 for wait. Each case records requests per
  second (best and median of the reps), p50, p99, errors, and the memory figures.

### 4. the Novis fact the table needs and nothing tests

**Does:** Adds the end-to-end test that one request's statics are not seen by the next under `nvs serve`.

File set: `crates/nvs-cli/tests/serve_isolation.rs` (new).

- **`a_static_set_by_one_request_is_not_seen_by_the_next`**: a served program sets a class static
  from the query and prints it. Many requests over one keep-alive connection and over several connections,
  on one worker and on several. Each response shows only its own value or the initial one.
- If it fails, that is a security bug and it is fixed in this stage, not reported in the table.

### 5. the feature scenarios

**Does:** Runs each feature row as a script against all four stacks and records what each one did, so the table is generated from results.

File set: `benches/compare/scenarios/<id>/` (one folder per row, one program per peer and a driver
step), `tools/nv/cmd/bench-compare.ts`.

Each row records, per peer, one of four results: `built-in`, `one-setting` (names the setting),
`your-code` (needs a library or code the developer writes) or `no`. It also records the measured
evidence beside the result: failed requests, seconds to apply, restarts. The rows, in this order:

| id | What runs |
|---|---|
| `cores` | throughput at one CPU against four, from stage 3's `fair` and `deployed` arms, in one server process |
| `isolation` | request A writes module-level or static state, request B reads it |
| `runaway` | one request loops forever and one allocates without end, under steady load; the others' errors and p99, and whether the backend restarted. Each peer uses its own limits if it has any; Novis sets `[limits]` and the row says `one-setting` |
| `crash` | a handler throws or panics; the next request's status, and container restarts |
| `escape` | the html-page term printed the way each language's documentation prints a value into HTML; raw `<script>` in the body or not |
| `taint` | a request value put into a SQL string: `nvs check` refuses it; the others have no such check, and the row says so with the language's own documentation linked |
| `secret` | a secret value printed: the same shape as `taint` |
| `csrf` | a cross-site POST with an `Origin` from another host, to a route that changes state: rejected or not |
| `headers` | the response headers of hello: `nosniff`, a frame policy and a referrer policy present or not |
| `waiting` | memory per request in flight, from the wait case |
| `parallel` | one request splits CPU work in four parts on a four-CPU budget, with each language's own way to use cores; time against one part |
| `live-edit` | the source changes and no command is run: seconds until the new answer, or never |
| `deploy` | under steady load, a 200-file tree is replaced by a copy with short pauses; failed requests and responses that mix the old and new version. Bun and Go deploy by rebuilding and restarting the container, as their docs describe |
| `config` | a response header set in each peer's config file changes: applied without a command, with a signal or command, or only after a restart |
| `upgrade` | the runtime binary itself is replaced under load: failed requests. Novis needs a restart here too, and the row shows it |

- `--scenarios --smoke` runs every row once at low load and prints `<id>: <result per peer>`.
- A row whose result surprises (Novis does worse than this file's facts say) is reported in the
  handoff, never edited away.

### 6. the page

**Does:** Copies the record to the website and adds the comparison page under In depth.

File set: `tools/nv/cmd/render.ts`'s website step, `website/src/data/compare.json`,
`website/src/lib/compare.ts` (new), `website/src/content/docs/in-depth/compare.mdx` (new), its
components under `website/src/components/compare/`, `website/astro.config.ts`'s sidebar.

- **Numbers as relations.** Each case is a bar chart scaled to the best peer in that case. The absolute
  figures are shown beside the bars, smaller. A block names the machine: CPU model, cores, OS, Docker
  version, commit and date. A sentence above the charts says that the absolute numbers depend on the
  machine and that the relations are the result.
- **The feature table** is generated from stage 5's rows. Every cell links to its scenario folder.
- **Run it yourself**: the page shows the one command and the tools it needs (`bun`, Docker).
- **Not the front door.** No figure from this page goes onto the landing page, the hero or the facts
  strip (`rule:programs/three-claims`).
- **The text** follows AGENTS.md § *Text an end user reads*. This page is its one stated exception
  for naming PHP, Bun and Go.

### 7. the first recorded run

**Does:** Takes the full run on this machine, records it in `benches/compare.json`, and renders it to the website.

File set: `benches/compare.json`, `website/src/data/compare.json`.

## Handoff

### State

**Side goal `compare-page`: every stage is done and every check is green.** `benches/compare.json` holds the first full run (2026-10-11, commit `1a413b584`, Ryzen 7 7800X3D, Docker 29.7.2). `website/src/data/compare.json` is its rendered copy. The site builds, and the page shows the record's figures and its 37 result cells. `bun nv verify --doc` is green, and `bun nv owners --closes compare-page` names no gap. The branch is ready to land on `main` by hand. This file is deleted as it lands.

- `bun nv render` also rewrites `data/schema/rule.schema.json` on this branch. That file is not this goal's, so the session restored it.

The recorded run. Cases: one cpu per backend, 20000 requests, three reps.

- hello: Bun 36623, Go 36539, Novis 31679 (0.86x), PHP 9183.
- json-api: Bun 34005, Novis 19123 (0.56x), Go 16806, PHP 5682.
- html-page: Bun 14776, Novis 7006 (0.47x), PHP 4916, Go 3167.
- wait: Bun 13962, Go 13868, Novis 13596 (0.97x), PHP 5145. Novis now sits at 0.97x of best, where the smoke run gave 0.39x.

Rows that differ from what the README leads the reader to expect, reported as measured:

- `runaway`: **Novis `no`.** `/loop` and `/alloc` were both unanswered, with 16 timeouts. That is the hang the earlier session saw at `memory = "16M"`, and it is now on the page.
- `parallel`: Novis `no` at 1.99x, Bun 3.67x, Go 3.36x, PHP 1x.
- `upgrade`: all four `no`. PHP's `php-fpm reload` dropped 13 requests in this run, where the smoke run dropped none. Novis dropped 3950 across its restart.
- `cores`: Bun `no` at 1.83x. Its four-cpu figure (32815) is near the front's ceiling.

### Next

**Nothing on this goal.** The user lands `side/compare-page` on `main`, deletes this file, and removes the worktree. The backlog below is what the run raised for the user to decide.

### Backlog

- `runaway`'s Novis hang, for the user: the recorded run shows it too. Reproduce it outside Docker with `[limits] memory = "16M"` and the `/alloc` loop, then say whether it is a runtime fault. A fix touches the runtime (`tools/nv/cmd/bench-compare.ts:865` is the row).
- `parallel`'s Novis 2x, for the user: whether the parent's parked core should take a child is a runtime decision (`crates/nvs-host/src/worker.rs:627`).
- `upgrade`'s PHP result changed between runs, from one-setting to `no` (13 `502`s). The row may need a tolerance, or PHP's reload really drops requests under load. Not checked.
- `cores`' Bun figure near the front's ceiling: give nginx and `oha` more cpus (`benches/compare/compose.common.yaml:18`, `benches/compare/nginx/http.conf:8`) if the page needs Bun's figure to measure Bun.
- The `taint` and `secret` documentation links are not checked; the page links each cell to its folder, not to them (`website/src/lib/compare.ts`).
