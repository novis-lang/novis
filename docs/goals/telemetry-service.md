---
looper: 1
title: The telemetry collector is a public Novis service that stores no address and publishes every total
summary: "apps/telemetry is a Novis web application that names no host, checks a report against the closed schema, adds it to weekly totals and never stores it, limits the rate on a keyed hash held in memory, writes no address to any log through the new [log] client_ip key, publishes every total on script-free pages and as CC0 JSON and CSV, and has a deploy guide; the real client uploads to it end to end on one machine, every designed rule this goal owns is shipped, and the checks below pass."
context: [apps/telemetry/README.md, apps/telemetry/src/App.nvs, apps/notes/src/App.nvs, docs/rules/tooling/the-telemetry-collector-is-a-novis-program.md, docs/rules/tooling/telemetry-counters-are-a-closed-set.md]
stages:
  - id: 1
    title: "`[log] client_ip = false` keeps the address out of every record the server writes"
    status: open
    context: [crates/nvs-config/src/tree.rs, crates/nvs-config/src/directive.rs, crates/nvs-server/src/trace.rs, crates/nvs-config/tests/directives.rs, docs/reference/tools/20-config.md, docs/rules/http-server/a-log-line-can-leave-out-the-client-address.md]
    extra:
      context:
        rules: [http-server/a-log-line-can-leave-out-the-client-address, http-server/the-access-log-is-a-mode-default]
  - id: 2
    title: the application exists, checks a report and adds it to the weekly totals
    status: open
    context: [apps/notes/src/App.nvs, apps/notes/src/Store.nvs, apps/notes/nvs.toml, apps/telemetry/src/Report.nvs, apps/telemetry/src/Totals.nvs, docs/rules/tooling/a-telemetry-report-is-checked-against-a-closed-schema.md, docs/rules/tooling/the-collector-keeps-weekly-totals-only.md]
    extra:
      context:
        rules: [tooling/the-telemetry-collector-is-a-novis-program, tooling/a-telemetry-report-is-checked-against-a-closed-schema, tooling/the-collector-keeps-weekly-totals-only, tooling/telemetry-counters-are-a-closed-set]
  - id: 3
    title: the rate limit runs on a keyed hash in memory, and no address reaches disk
    status: open
    context: [apps/telemetry/src/Limit.nvs, apps/telemetry/nvs.toml, docs/rules/tooling/the-collector-never-stores-an-address.md, docs/rules/core-classes/ratelimit-two-members.md]
    extra:
      context:
        rules: [tooling/the-collector-never-stores-an-address, core-classes/ratelimit-two-members, core-classes/ratelimit-gcra]
  - id: 4
    title: the public pages and the data downloads
    status: open
    context: [apps/telemetry/src/Pages.nvs, apps/telemetry/src/Charts.nvs, apps/telemetry/src/Data.nvs, apps/notes/src/Pages.nvs, docs/rules/tooling/telemetry-totals-are-public.md]
    extra:
      context:
        rules: [tooling/telemetry-totals-are-public, tooling/the-collector-keeps-weekly-totals-only]
  - id: 5
    title: the deploy guide, the known list follows the binary, and the reference names the collector
    status: open
    context: [apps/telemetry/README.md, apps/telemetry/known.toml, crates/nvs-cli/src/telemetry.rs, crates/nvs-cli/tests/telemetry_known.rs, docs/reference/tools/10-cli.md]
    extra:
      context:
        rules: [tooling/the-telemetry-collector-is-a-novis-program, tooling/a-telemetry-report-is-checked-against-a-closed-schema]
  - id: 6
    title: the real client uploads to the collector end to end on one machine, and every rule is shipped
    status: open
    context: [crates/nvs-cli/tests/telemetry_e2e.rs, crates/nvs-cli/src/telemetry.rs, crates/nvs-cli/src/endpoints.rs, apps/telemetry/README.md]
    extra:
      context:
        rules: [tooling/the-telemetry-collector-is-a-novis-program, tooling/a-failed-upload-never-fails-the-command, tooling/telemetry-show-prints-the-payload]
  - id: 7
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: client-ip-proofs
    stage: 1
    name: the log.client_ip directive has its feature proofs
    run: [bun, nv, proofs, --gate, --only, "directive:log.client_ip"]
  - id: app-tests
    stage: 2
    name: the collector's tests pass
    run: [cargo, run, -q, --bin, nvs, --, test, apps/telemetry]
  - id: end-to-end
    stage: 6
    name: the real client uploads to a local collector and the totals show it
    run: [cargo, test, -q, --test, telemetry_e2e]
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, telemetry-service]
    reach: true
    floor: false
---

# The telemetry collector is a public Novis service that stores no address and publishes every total

## Why

Goal `cli-housekeeping` builds the client: `nvs telemetry` records a closed set of counters, and an
upload is carried by a short command once a week. It ships with no default upload URL, because no
collector exists. This goal builds the collector, so opted-in reports have somewhere to go. The
collector is small, public and open source, and anybody can read the same totals the project reads.
It runs on Novis, so it is also a real program serving traffic Novis does not control.

## Standing decisions

**What a report carries is settled.** The user decided the payload: the version, the
system, the week and the `invocation.<subcommand>` counters already designed, plus
`internal-error.<subcommand>`, `days_active` and `first`. Compiler diagnostic codes were declined. The
payload's home is `rule:tooling/telemetry-counters-are-a-closed-set`, and goal `cli-housekeeping`
builds it. This goal only reads it.

**The address is kept out of every log by a Novis setting.** The user chose this over hiding the
address in the proxy. `[log] client_ip = false` is a `System` directive that every Novis
operator can use (`rule:http-server/a-log-line-can-leave-out-the-client-address`). Its row goes beside
`log.handler` in `crates/nvs-config/src/directive.rs:175`, its key in `Log`
(`crates/nvs-config/src/tree.rs:575`), and its reader in `access`
(`crates/nvs-server/src/trace.rs:371`), which leaves `clientIp` out of the line instead of writing
`null`.

**The totals are a week by version by system cross-table, all public.** The user chose this
(`rule:tooling/the-collector-keeps-weekly-totals-only`).

**The goal runs right after `cli-housekeeping`.** The user chose this, so the client and its
collector ship close together.

**The application is `apps/telemetry/`, in the shape of `apps/notes/`.** It stores its totals in
`telemetry.sqlite` through `Core\Db`, with a `[db.scratch]` in-memory database for tests, as
`apps/notes/nvs.toml` does. `public/index.nvs` is the entry, and `src/App.nvs` routes. The files are
`Report.nvs` (the check), `Totals.nvs` (the two tables), `Limit.nvs` (the rate limit), `Pages.nvs`,
`Charts.nvs` and `Data.nvs` (the public side), `known.toml` (the closed lists), and a `*Test.nvs`
beside each. The schema is created when the first request opens the database, as in `apps/notes/`.
Why: simplicity; the notes app already shows every piece this needs.

**The report route is public and opts out of CSRF on the route.** It is
`#[Core\Access(allow: Core\Audience::Public, csrf: false)]`, as
`docs/examples/types/Audience/03-a-webhook-another-server-calls.nvs:14` shows. The application has no
cookie and no session, so a forged request has nothing to ride on, and `nvs.toml` names no
`csrf_key` (`rule:security/csrf-is-on-by-default`). Every other route is `GET`.

**`known.toml` is the collector's copy of the closed lists.** It lists the commands, the `os` values
and the `arch` values. A Rust test, `crates/nvs-cli/tests/telemetry_known.rs`, reads it and fails
when the binary's command table names a command it does not list. Why: the collector runs one version
of Novis and receives reports from many, so it cannot read the table at run time.

**The clock is Core's, so tests move it.** The week window and the daily salt read Core's clock,
never SQLite's `now`, and tests set it with `#[Test(at: ...)]` and `Core\Test::advance`
(`rule:testing/determinism-declared-on-the-test`).

**Charts are SVG built by `Charts.nvs`, with no library.** Two shapes cover every chart on the pages:
a line over weeks and a horizontal bar list. Why: the pages promise no script
(`rule:tooling/telemetry-totals-are-public`), and two shapes are a small amount of code.

**The domain is configuration, and nothing waits on it.** The program builds every link and endpoint
from the request's own scheme and host, so it names no host
(`rule:tooling/the-telemetry-collector-is-a-novis-program`). The domain appears in two places, both
outside the program: the Caddyfile's site name, written as `telemetry.example.com` in the guide, and
the client's `NVS_TELEMETRY_URL`. The client's default URL stays empty in this goal
(`rule:config/telemetry-and-update-endpoints-are-configuration`). Setting it when a collector is
deployed is one constant in `crates/nvs-cli/src/endpoints.rs`. Why: the whole service is built and
tested on one machine, and the deployment changes no code.

**The end-to-end test is the real client against the real application.**
`crates/nvs-cli/tests/telemetry_e2e.rs` starts the `nvs` binary as `nvs serve` on the application with
a temporary copy of its `nvs.toml` pointing `[db.telemetry]` at a temporary file, on a free loopback
port. It writes a data folder with consent on and a finished, unsent week, and runs `nvs telemetry
upload` with `NVS_TELEMETRY_URL` at the server. The test then checks the `204`, the row in
`/data/totals.json`, the line on `/`, a second upload in the same week refused locally, a malformed
report dropped on `400`, and that `127.0.0.1` is in no table and no log line the server wrote. Why:
it is the one test that proves the client's payload and the collector's schema agree.

**The deployment is Caddy in front of `nvs serve`, and the guide is the whole of it.** Caddy holds
the certificate for the domain and proxies to `127.0.0.1:8000`. Its Caddyfile has no
`log` directive, so it writes no access log. The application's `nvs.toml` sets `[mode] production`,
`[log] client_ip = false` and `[server] trusted_proxies = ["127.0.0.1"]`. The deploy step writes the
commit to `apps/telemetry/REVISION`, which the footer reads, and the footer says `unknown` when the
file is missing. `REVISION` is in `.gitignore`. Why: Caddy is one binary with automatic TLS, and a
guide in the repository makes the deployment as public as the code.


**Lanes.** Stage 1 is Rust in `nvs-config` and `nvs-server` and is disjoint from the application, so
it can run beside stage 2. Stages 2, 3 and 4 share `src/App.nvs` and the schema and run in order.
Stage 3 needs stage 1's key for its log test. Stage 5 touches `README.md`, `known.toml`, one new Rust
test and the CLI reference, and runs after 4. Stage 6 is last.

**Gates.** Stage 3's log test needs stage 1:
`data/rules/http-server/a-log-line-can-leave-out-the-client-address.json` reads `"status": "shipped"`.
Until then, the session takes stage 3's other items. Stage 5's known-list test and its reference
section need goal `cli-housekeeping` reached: `docs/goals/cli-housekeeping.md` is gone. Stage 6 needs
the same, because it runs the real client. The chain puts this goal after `cli-housekeeping`, so the
gate holds whenever this goal is live. A session whose stage is gated takes the earliest open stage
whose gate holds, and ends `BLOCKED` naming the missing file when none does.

**Trade-offs.**
- No language feature changes, and the request path of other programs is untouched.
- `[log] client_ip` is one new `System` key. It removes one field from a record that is written
  anyway, so it costs nothing at run time.
- The collector spends per request one report of at most 4 KiB and one transaction. Per process it
  holds one GCRA timestamp per address seen in the last hour and one salt.
- The database grows with weeks, versions, systems and commands, never with reports.
- Security and anonymity are the reason for every choice. There is no identifier and no stored
  address. A raw report is never written down, and every total is public.
- The cost is that a flood from many addresses can inflate the totals. The value cap bounds that, and
  the rules state it.

## Stages

### 1. `[log] client_ip = false` keeps the address out of every record the server writes

**Does:** a new `System` directive leaves `clientIp` out of the access line and the `5xx` line.

- [ ] The `client_ip` key in `Log`, and its row in the directive table. `crates/nvs-config/src/tree.rs:575`, `crates/nvs-config/src/directive.rs:175`
- [ ] `access` leaves the field out when the key is `false` (`rule:http-server/a-log-line-can-leave-out-the-client-address`). `crates/nvs-server/src/trace.rs:371`, `crates/nvs-server/src/trace.rs:410`
- [ ] Tests: a `5xx` line with the key `false` has no `clientIp`, and with it unwritten it has one. `crates/nvs-server/src/trace.rs:909`
- [ ] The directive's Rust test, beside `log.handler`'s. `crates/nvs-config/tests/directives.rs:1`
- [ ] The key in the `[log]` row of the config reference. `docs/reference/tools/20-config.md:65`
- [ ] The directive's feature proofs: `docs/examples/config/log-client_ip/about.md`, one example and one attack, whose step tries to turn the key back on with `Core\Config::set`. `bun nv proofs --id directive:log.client_ip` prints the paths. `docs/examples/config/log-client_ip/about.md:1`, `tests/hostile/config/log-client_ip/01-turning-the-address-back-on.nvs:1`
- [ ] The rule `shipped`, with its tests in `guardedBy`. `data/rules/http-server/a-log-line-can-leave-out-the-client-address.json:3`

### 2. the application exists, checks a report and adds it to the weekly totals

**Does:** `POST /v1/report` checks the body against the closed schema, and an accepted report is
added to the two tables in one transaction.

- [ ] `apps/telemetry/` with `README.md`, `nvs.toml`, `known.toml`, `public/index.nvs` and `src/App.nvs`, in the shape of `apps/notes/` (`rule:tooling/the-telemetry-collector-is-a-novis-program`). `apps/notes/src/App.nvs:1`, `apps/telemetry/src/App.nvs:1`
- [ ] The check: every field in the rule's table, the `unknown` folding, the value cap, and the status per refusal (`rule:tooling/a-telemetry-report-is-checked-against-a-closed-schema`). `apps/telemetry/src/Report.nvs:1`
- [ ] The two tables, and the transaction that adds one report (`rule:tooling/the-collector-keeps-weekly-totals-only`). `apps/telemetry/src/Totals.nvs:1`
- [ ] Tests for the check: each field wrong once, a name `known.toml` lacks, a value over the cap, a week in the future, a week five weeks old, 4 KiB plus one byte, a wrong content type. `apps/telemetry/src/ReportTest.nvs:1`
- [ ] Tests for the totals: two reports in one cell, `first` and `days_active` counted, a counter's `reports` against its `total`, and a refused report changes nothing. `apps/telemetry/src/TotalsTest.nvs:1`

### 3. the rate limit runs on a keyed hash in memory, and no address reaches disk

**Does:** `Core\RateLimit::shed` limits each address to ten reports an hour, keyed on an HMAC under a
daily salt, and the application's configuration writes no address.

- [ ] The salt in `Core\Cache::memory()` until the end of the UTC day, the HMAC key, `shed`, and `429` with `Retry-After` (`rule:tooling/the-collector-never-stores-an-address`). `apps/telemetry/src/Limit.nvs:1`
- [ ] `nvs.toml` sets `[log] client_ip = false` and `[mode] production`. `apps/telemetry/nvs.toml:1`
- [ ] Tests: the eleventh report in an hour gets `429`, a new hour accepts again, and a new day uses a new key. `apps/telemetry/src/LimitTest.nvs:1`
- [ ] The test that sends reports from a known address, then reads every table and every log line, and finds the address in none (gated on stage 1). `apps/telemetry/src/LimitTest.nvs:1`

### 4. the public pages and the data downloads

**Does:** every path in `rule:tooling/telemetry-totals-are-public`'s table, as HTML with inline SVG
and as JSON and CSV.

- [ ] The two chart shapes as SVG, with no script. `apps/telemetry/src/Charts.nvs:1`
- [ ] `/`, `/collect` and `/data`, the footer with the commit from `REVISION`, and the four newest weeks marked as still arriving (`rule:tooling/telemetry-totals-are-public`). `apps/telemetry/src/Pages.nvs:1`
- [ ] `/data/totals.json`, `/data/totals.csv` and `/data/<week>.json`. `apps/telemetry/src/Data.nvs:1`
- [ ] The `Content-Security-Policy` header, and no `Set-Cookie` on any answer. `apps/telemetry/src/App.nvs:1`
- [ ] Tests: each page with an empty database and with two weeks of totals, the CSP header, no cookie, the JSON and the CSV rows equal to the tables, and an unknown week `404`. `apps/telemetry/src/PagesTest.nvs:1`, `apps/telemetry/src/DataTest.nvs:1`
- [ ] The page text follows `AGENTS.md` § *Text an end user reads*. `apps/telemetry/src/Pages.nvs:1`

### 5. the deploy guide, the known list follows the binary, and the reference names the collector

**Does:** a person can deploy the collector from its README alone, and a test keeps `known.toml` in
step with the command table.

- [ ] The local run: `nvs serve public/index.nvs`, then `NVS_TELEMETRY_URL=http://127.0.0.1:8000/v1/report nvs telemetry upload` from another terminal, and the totals on `http://127.0.0.1:8000/`. `apps/telemetry/README.md:1`
- [ ] The deploy guide: Caddy and its Caddyfile with no `log` and `telemetry.example.com` as the site, `nvs.toml`, `REVISION`, how to run it as a service under its own operating-system account, so it shares a host with the registry and cannot read the registry's files, and the one constant to set for the client's default URL (`rule:tooling/the-telemetry-collector-is-a-novis-program`). `apps/telemetry/README.md:1`
- [ ] `REVISION` in `.gitignore`. `apps/telemetry/.gitignore:1`
- [ ] The test that every command in the binary's table is in `known.toml` (gated on `cli-housekeeping`). `crates/nvs-cli/tests/telemetry_known.rs:1`, `crates/nvs-cli/src/telemetry.rs:1`
- [ ] `# nvs telemetry` in the CLI reference says the collector is `apps/telemetry/` and how to run one, and lists the answers and what the client does with each (gated on `cli-housekeeping`). `docs/reference/tools/10-cli.md:509`

### 6. the real client uploads to the collector end to end on one machine, and every rule is shipped

**Does:** one Rust test runs `nvs serve` on the application and the real `nvs telemetry upload`
against it, on loopback, as § *Standing decisions* describes.

- [ ] The test starts the server on a free port with a temporary database, and stops it at the end. `crates/nvs-cli/tests/telemetry_e2e.rs:1`
- [ ] The upload of a finished week: `204`, the row in `/data/totals.json`, and the line on `/` (`rule:tooling/the-telemetry-collector-is-a-novis-program`). `crates/nvs-cli/tests/telemetry_e2e.rs:1`
- [ ] A second upload in the same week refused by the client, and a report the collector refuses with `400` dropped (`rule:tooling/a-failed-upload-never-fails-the-command`). `crates/nvs-cli/tests/telemetry_e2e.rs:1`
- [ ] The bytes `nvs telemetry show` printed are the bytes the server received (`rule:tooling/telemetry-show-prints-the-payload`). `crates/nvs-cli/tests/telemetry_e2e.rs:1`
- [ ] `127.0.0.1` is in no table and no log line the server wrote (`rule:tooling/the-collector-never-stores-an-address`). `crates/nvs-cli/tests/telemetry_e2e.rs:1`
- [ ] The five tooling rules `shipped`, each with its tests in `guardedBy`. `data/rules/tooling/the-telemetry-collector-is-a-novis-program.json:3`, `data/rules/tooling/a-telemetry-report-is-checked-against-a-closed-schema.json:3`, `data/rules/tooling/the-collector-keeps-weekly-totals-only.json:3`, `data/rules/tooling/the-collector-never-stores-an-address.json:3`, `data/rules/tooling/telemetry-totals-are-public.json:3`
- [ ] `bun nv owners --closes telemetry-service` passes.

### 7. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

No stage is started. The user settled the payload, the address setting, the totals' shape and the
place in the chain, and the rules are written as `designed`. The anchors above were
checked against the tree.

### Next

**Stage 1, `[log] client_ip = false`**, beside **stage 2, the application**: disjoint files, no gate
on either.

- [ ] The key, its directive row, and `access` leaving the field out. `crates/nvs-config/src/tree.rs:575`, `crates/nvs-config/src/directive.rs:175`, `crates/nvs-server/src/trace.rs:371`
- [ ] Its tests, its reference row and its feature proofs. `crates/nvs-config/tests/directives.rs:1`, `docs/reference/tools/20-config.md:65`
- [ ] `apps/telemetry/` with the check and the totals, and their tests. `apps/telemetry/src/Report.nvs:1`, `apps/telemetry/src/Totals.nvs:1`

### Backlog

- Deploying the collector and setting the client's default URL to it are not this goal's work. The
  default is one constant in `crates/nvs-cli/src/endpoints.rs`, set in the same change that updates
  `rule:config/telemetry-and-update-endpoints-are-configuration`.
- Download counts from GitHub releases on the public pages would correct the opt-in undercount
  (`rule:tooling/telemetry-is-two-opt-ins`). They need an outbound job, and no goal owns it.
