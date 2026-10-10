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
    status: open
    extra:
      context:
        rules: [config/an-edit-reaches-the-next-request-without-a-restart, config/a-request-keeps-the-unit-it-resolved, core-classes/html-auto-escape]
  - id: 6
    title: the page
    status: open
    context: [tools/nv/cmd/render.ts, website/astro.config.ts]
  - id: 7
    title: the first recorded run
    status: open
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
    output_contains: ["\"novis\"", "\"php\"", "\"bun\"", "\"go\"", "\"host\"", "\"scenarios\""]
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

**Side goal `compare-page`, stage 5 in progress: 13 of 15 rows run.** `bun nv bench-compare --scenarios --smoke` exits 0 in Docker and prints a line per row from `SCENARIOS`. `--row <id>` runs one row. Each row maps what a peer did to a `Verdict` through a pure function, unit-tested in `tools/nv/test/bench-compare.test.ts`. A row either has a `probe` (a `Probe` against one stack: `get` with an optional timeout, `send`, `running`, `check`, `exec`, `logs`, `steady` — `oha -z` started without waiting, parsed by `steadyResult` — `peer`, `dir`) or is `timed` (a `Timer` whose `pass(cpus)` is one fresh `onePass` of the case named by `case`, with the run's `--requests` and `--reps`). A row may give a peer a `fixed` result, so that peer runs nothing; `serves: false` brings no stack up; `copy: true` serves a copy under `generated/`; `memory` caps the backend container (`BACKEND_MEMORY`, memory and swap, `0` elsewhere); `cpus` overrides the run's backend budget. A Novis row that needs a setting writes it with a `TableEdit` (`tableEditCommand`: a `sed` over one table of the data folder's `nvs.toml`) and waits for `configuration reloaded`; the unit test checks the shipped file's shape without a host `sed`, which the driver's PATH lacks. `benches/compare/README.md` § *The feature scenarios* is why.

Results so far:

- `cores` **is a surprise, and a measurement fault, not a Novis fact.** At `--requests 20000 --reps 1`: Novis 27061 to 35913 (1.33x, `no`), PHP 10025 to 34309 (3.42x, built-in), Bun 32883 to 35160 (1.07x, `no`), Go 14703 to 15484 (1.05x, `no`). Every four-cpu figure sits near 35k, which is nginx's and `oha`'s ceiling at four cpus each, so the row measures the front and not the backends. At `--smoke`'s 300 requests all four read `no`. Reported, not adjusted; see Next.
- `waiting` (smoke, 300 in flight): Novis built-in 59.8 KiB per request, PHP one-setting (`pm.max_children`) 3232.5 KiB, Bun built-in 34.7 KiB, Go built-in 32.8 KiB. No failed request.

- `runaway` **is a surprise, and looks like a Novis fault.** Smoke, 512 MiB container with no swap, Novis `[limits] memory = "16M"`, `cpu_time = "1s"` (the `configuration reloaded` line names both as applied): Novis `no`. Neither `/loop` nor `/alloc` was answered in 55 s, the load's last 16 requests timed out, and the container still ran. PHP built-in (loop stopped at 38.9 s, p99 65.8 ms, no failure). Bun `no` (the loop blocks its one process). Go `no` (`/alloc` OOM-kills the container, 406 `502`s). At `memory = "64M"` Novis stopped `/loop` with `500` in 1.3 s, but admitted only 6 requests ("the memory budget affords 6 requests"), so `/alloc` got `503` and 140 load requests failed. So the hang follows `/alloc` actually running: an array growing by `Core\Str::repeat("x", 1048576)` (`benches/compare/scenarios/runaway/main.nvs`) is not stopped by `[limits] memory`, and it stalls the other requests too. Not checked: whether the cap is never charged or the stop never lands. Reported, not adjusted; see Next.
- `isolation`: Novis built-in, PHP built-in, Bun no, Go no (19 of 20 requests saw another's value).
- `crash`: all four built-in. PHP answers the throw with `200`, and Go with `502`.
- `escape`: Novis and Go built-in, PHP and Bun your-code.
- `taint`: Novis built-in (`nvs check` fails with E0401), the other three no.
- `secret`: Novis built-in (E0790), the other three no.
- `csrf`: Novis built-in (`403`), the other three your-code (`200`).
- `headers`: Novis built-in, the other three your-code.
- `parallel` **is a surprise, and follows a runtime rule.** Smoke, four backend cpus, 200M loop turns a part, fastest of 3: Novis `no` (0.657 s for one part, 1.337 s for four, 1.97x; it was also 2.0x at 20M turns, so not start-up cost), PHP `no` (1.01x, parts in a row), Bun built-in (3.87x, `Worker`), Go built-in (3.9x, goroutines). Novis also needs `[capabilities.script] spawn = true`, so its best case is `one-setting`, not the tie the standing decisions expected. Why 2x: `WorkerCores::pick` (`crates/nvs-host/src/worker.rs:627`) never places a child on its parent's core and never grows past the registered serving cores, so on four cores three children get a core each and the fourth queues behind one, while the parent's core sits parked. Not checked: that `nvs serve` started exactly four workers in the container. Reported, not adjusted.
- `live-edit`: Novis built-in (1 s), PHP one-setting (`opcache.validate_timestamps=1`), Bun one-setting (`bun --hot`), Go no.
- `config`: Novis built-in (2.8 s, no command), PHP one-setting (`php-fpm reload`, 0.5 s after USR2), Bun and Go your-code.

The documentation links in `taint` and `secret`'s fixed results are not checked: no session has opened them. The `every-scenario-runs` check stays red until all 15 row ids are printed. It now fails on `deploy:`, the next row not written, not a regression.

### Next

**Stage 5: `deploy`, and the three surprises.** File set `benches/compare/scenarios/`, `tools/nv/cmd/bench-compare.ts`. Goal prose § *Stage 5* is what each row runs; rule `testing/userland-benchmarks`.

- [ ] **`deploy`**: under steady load, a 200-file tree replaced by a copy with short pauses; failed requests and mixed versions. Bun and Go rebuild and restart. A new folder and a row in `SCENARIOS` after `live-edit` (`tools/nv/cmd/bench-compare.ts:784`); `copy: true`, `p.dir` and `p.steady` exist.
- [ ] **`parallel`'s Novis 2x, for the user.** The row is `tools/nv/cmd/bench-compare.ts:772`, the placement `crates/nvs-host/src/worker.rs:627`. Whether the parent's parked core should take a child is a runtime decision; the standing decisions keep the runtime unchanged, so ask before touching it.
- [ ] **`runaway`'s Novis hang, for the user.** The row is `tools/nv/cmd/bench-compare.ts:691`. Reproduce it outside Docker with `[limits] memory = "16M"` and the `/alloc` loop of `benches/compare/scenarios/runaway/main.nvs`, then say whether it is a runtime fault. The standing decisions allow a security fix. Fixing it is a decision for the user, since it touches the runtime.
- [ ] **`cores`' front ceiling**: nginx and `oha` have four cpus each (`benches/compare/compose.common.yaml:18`, `:43`, `benches/compare/nginx/http.conf:8`). Give them enough room for the four-cpu pass, or time a heavier case, so the figure measures the backend. Re-run `--row cores --requests 20000` and report the result as measured.

### Backlog

- The last load row, `upgrade`, goal prose § *Stage 5*.
- `--scenarios` refuses `--record` for now. Stage 7's record needs the rows under a `scenarios` key in `buildRecord`.
- Stage 6: `website/src/content/docs/in-depth/compare.mdx`, goal prose § *Stage 6*.
- `wait`: Novis reached 0.39x of best at 1000 connections on one CPU, with no `503`. The page reports it as measured, since the standing decisions keep the runtime unchanged. Why it is slower is not checked.
