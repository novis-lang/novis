---
milestone: M7
---
# Loop goal 57 — everything M7 promised a deployment is there to run

A server started with `nvs serve` can be operated the way M7 said it would be. An operator reloads it,
reads its live configuration and asks its status with `nvs ctl`. They install it under the platform's own
service manager with `nvs service`, and stop it with a drain rather than a kill. They scrape its metrics
and receive its traces. A program answers with every body member the spec lists, including `html`
and `sendFile`, and `echo` escapes into an HTML response. It reads a binary body whole. It links
through its mount and its enum captures, and its tests send headers and a body. A fleet schedule fires
once across the deployment, and a runaway script under `nvs serve` is ended as a `FATAL` while the
core goes on serving.

## Why here

After goal `m4b-editor` and before goal `m8-db-queue`, because the gap program closes milestones in
order: M7's promises are owed before M8's, and goal `m8-stdlib-depth`'s `Core\Metrics` rows have
nothing to be read through until this goal's exporter runs. Goal `plan-truth` and goal `gap-register`
run first so that the anchors below are read from documents that are true. Goal `unowned-closures`
runs later, behind a decision sheet, and takes the `unowned` gaps this goal leaves alone.

What it needs already built:
- the accept loop and its drain state: `crates/nvs-server/src/serve.rs:1726` `serve_on_this_core`,
  whose `keep_serving` closure is the stop, and `Draining` at `:473`;
- the control endpoint's creation and trust check: `crates/nvs-config/src/control.rs:150` and `:241`;
- the reload's publish and report: `crates/nvs-config/src/control.rs:276`, `Current` at
  `crates/nvs-config/src/snapshot.rs:315`;
- the operation roster: `crates/nvs-server/src/control.rs:26`;
- the service plan and its refusals: `crates/nvs-cli/src/service.rs:140`, `:372`, `:553`;
- the schedule ticker's `Leases` seam: `crates/nvs-server/src/schedule.rs:301`;
- the per-core registry: `crates/nvs-server/src/metrics.rs:368`;
- the door's trace identity: `crates/nvs-server/src/trace.rs`;
- `Core\Html\Markup` as a registry type: `crates/nvs-stdlib/src/html.rs:215`;
- the in-process request seam: `crates/nvs-runtime/src/inproc.rs`;
- the shared tier's Redis client: `crates/nvs-stdlib/src/cache/redis.rs`.

## Stage 0 — the catch-up

These sentences on disk become wrong under this goal. Each is rewritten whole, in the stage that makes
it wrong. Re-grep before editing: these are anchors, and files move.

- `crates/nvs-server/src/control.rs:9-13` — "`reload` is the only operation". Stage 3.
- `crates/nvs-cli/src/main.rs:740-743` — `ctl config` "arrives with `nvs ctl`". Stage 3.
- `crates/nvs-cli/src/main.rs:911-912` — why `install` and its siblings are not here. Stage 5.
- `crates/nvs-cli/src/service.rs:51-60` — "Registration itself has not landed". Stage 5.
- `crates/nvs-cli/src/serve.rs:543-546` and `:824-834` — nothing asks the loop to stop, and an instance
  ends by being killed. Stage 3.
- `crates/nvs-cli/src/serve.rs:57-62` — a Unix-domain entry is refused. Stage 4.
- `crates/nvs-cli/src/serve.rs:753-761` — `None` for the lease. Stage 9.
- `crates/nvs-stdlib/src/response.rs:7-17` and gap 1 at `:191-201`; `crates/nvs-stdlib/src/html.rs:83-90`.
  Stage 6. `response.rs:13-14`'s reason for `html`'s absence is already false, because
  `html.rs:154` puts `Markup` in a registry row today.
- `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:30-31` — two keys owned by the retired
  goal `test-request`. Stage 6 strikes them.
- `docs/spec/01-core-library.md:1122` — `sendFile(…)`. The stage 2 record spells it.
- `docs/spec/01-core-library.md:1101-1102` — `bodyStream` as the raw-body alternative. Stage 7.
- `docs/spec/01-core-library.md:1001` — `request`, one bare word. Stage 7.
- `crates/nvs-stdlib/src/test.rs:141-147` — a synthetic request with no headers and no body. Stage 7.
- `crates/nvs-types/src/links.rs:51-57`, `crates/nvs-types/src/routes.rs:1781-1786`, and the last
  sentence of `docs/rules/routing/a-capture-narrows-to-a-closed-set.md` — an enum case with no segment
  spelling. Stage 2 decides the spelling and stage 8 builds it.
- `crates/nvs-stdlib/src/router.rs:63-72`, `crates/nvs-config/src/mount.rs:28`, and the *Not shipped*
  paragraph of `docs/rules/routing/an-origin-is-per-mount-and-checked-at-boot.md`. Stage 8.
- `crates/nvs-server/src/schedule.rs:51-62` and `:77-91`, and the "Today `nvs serve` supplies none"
  paragraph of `docs/rules/config/a-fleet-entry-fires-at-most-once-under-a-lease.md`. Stage 9.
- `crates/nvs-server/src/metrics.rs:58-73` (both gaps), `crates/nvs-server/src/route.rs:46-56` (gap 2), the two
  `[unread: …]` trailers at `crates/nvs-config/src/tree.rs:1020` and `:1024`, and
  `crates/nvs-config/src/default.toml:725`. Stages 10 and 11.
- `crates/nvs-runtime/src/trace_context.rs:22-23` and `crates/nvs-server/src/trace.rs:20-25`, which say
  nothing exports or creates a span. Stage 11.
- `docs/plan/m6.md:41-42` and `:50` (the reload client and `nvs ctl config` "arrive with M7"), and
  `docs/plan/m7.md:88-92` (*Not yet named here*). Rewrite them with `python tools/plan.py --amend`, in
  stages 3 and 7.

## Stage 1 — the floor

Goal `m4b-editor`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the record

One new record, and no other number. It holds the calls that no current rule states, argued under
ADR 0004's ordering. It creates two rules, both `designed`:

| Rule | Says |
|---|---|
| `routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name` | a backed enum's case is its backing value in a path segment and in a query value; a pure enum's case is its case name, compared case-sensitively; a segment naming no admitted case falls through to `404` by the failed-conversion rule |
| `observability/an-exporter-brings-no-second-scheduler-and-no-second-client` | the scrape endpoint is served by this server's own accept loop, and a push goes through `nvs-host`'s stream and its one TLS client. A crate is taken only for a half that needs neither — an encoder, never a transport |

It modifies `routing/a-capture-narrows-to-a-closed-set`, whose last sentence goes, and
`observability/the-exporters-are-crates`, which keeps "somebody else's specification" and loses "taken
as dependencies" wherever a dependency would bring its own runtime or client. **Before writing the
second rule**, check `Cargo.lock` and each candidate crate's own manifest for what `metrics-exporter-prometheus` and
`opentelemetry-otlp` pull in. If either can be taken with no async runtime and no HTTP or TLS client of
its own, the rule says "taken" for that half and the record says why. The record also spells
`Core\Response::sendFile` in spec § 15 (Stage 6's signature) and names the drain bound Stage 3 uses.

## Stage 3 — the keystone: the control socket, `nvs ctl`, and the drain

`crates/nvs-server/src/control.rs`, `crates/nvs-config/src/control.rs`, `crates/nvs-cli/src/main.rs`,
`crates/nvs-cli/src/serve.rs`, and a new `crates/nvs-cli/src/ctl.rs`. Everything after this stage
operates a server through this stage's work. `nvs service status` asks the socket, `PARAMCHANGE`
calls the reload, a stop is the drain, and the end-to-end stage stops what it started.

- **`nvs serve` binds `[control] socket`** with `bind` and `boundary` (`crates/nvs-config/src/control.rs:241`)
  before any listener accepts, and refuses the boot on a refusal. A tree with no `[control]` block gets no
  endpoint, per `rule:config/one-local-control-socket`.
- **The endpoint answers three operations**, and `Operation` (`crates/nvs-server/src/control.rs:26`) gains the
  two new variants:
  - `POST /reload` publishes through `reload` (`crates/nvs-config/src/control.rs:276`), passing the
    unit cache's count as `held`, answers the `Report` and writes it to `Core\Log`;
  - `GET /config` answers the live snapshot with each directive's origin, as `nvs config dump --origin`
    prints it (`rule:config/ctl-config-reports-the-live-snapshot`);
  - `GET /status` answers the in-flight count and whether the process is draining — the reading
    `rule:packaging/a-service-is-one-stored-argv` says `status` asks for.
- **Every answer carries the server's version** in one response header.
- **`nvs ctl reload | config [--origin] | status [--socket <path>]`** is the client, and refuses an
  answer from another version, naming both.
- **The drain.** `nvs serve` installs the terminating-signal handler: `sigaction` for `SIGTERM` and
  `SIGINT` on Unix, `SetConsoleCtrlHandler` on Windows. Both libraries are already dependencies. The
  handler's whole effect is `Drain::process().begin()` (`crates/nvs-stdlib/src/signal.rs:42-44`).
  `keep_serving` then answers `Break` once the drain has run out, and every connection closes
  under `rule:concurrency/a-drain-closes-a-connection-cleanly`.
- **`sd_notify`.** When `NOTIFY_SOCKET` is set, `nvs serve` sends:
  - `READY=1` once every listener is bound and every entry is compiled;
  - `RELOADING=1` with `MONOTONIC_USEC` and then `READY=1` around a reload;
  - `STOPPING=1` when the drain begins.

  It is written by hand over a Unix datagram socket, with no crate. The unit
  `crates/nvs-cli/src/service.rs:503` renders is `Type=notify`, and without this line systemd stops it
  at `TimeoutStartSec`.

## Stage 4 — the Unix listener

`crates/nvs-host/src/net.rs` (`NvsListener`) and `crates/nvs-cli/src/serve.rs:57-62`.
`rule:http-server/a-unix-socket-listener`: a `listen` entry beginning with a separator is a Unix socket,
Unix only, created with `socket_mode`. On Windows the refusal stays, and it is still taken once over the
whole set before any socket exists. The listener accepts on the same reactor as TCP, so nothing about a
connection differs past `accept`. This stage does not share files with stage 3; give it its own session.

## Stage 5 — `nvs service`

`crates/nvs-cli/src/service.rs`, `crates/nvs-cli/src/main.rs:914` (`ServiceCommand`), and the root
`Cargo.toml`'s `windows-sys` features: add `Win32_System_Services` and `Win32_System_EventLog` beside
`:186-208`.

- **`install`, `uninstall`, `start`, `stop`, `status` and `run`** as `rule:packaging/a-service-is-one-stored-argv`
  lists them, and `nvs install-service` as the hidden alias.
- **Windows install**: SCM registration with `ImagePath` from `image_path` (`:372`), the per-service virtual
  account, `PRESHUTDOWN` requested, failure actions (`--restart on-failure`), `--start` for delayed
  auto-start, `--depends-on`, a description, and the event-log source.
- **Linux install**: the unit `unit()` renders, written to `destination` (`:553`), then `systemctl
  daemon-reload`.
- **`uninstall`** leaves no key, no source, no unit and no ACL (`rule:packaging/a-service-answers-its-manager`).
- **`start`, `stop` and `status`** go to the SCM directly or to `systemctl` by argv with no shell.
  `status` adds stage 3's `GET /status`.
- **`run`** is the SCM's entry point. It maps `STOP` and `PRESHUTDOWN` to the drain, reporting
  `STOP_PENDING` with a checkpoint that advances while requests remain. It maps `PARAMCHANGE` to the
  reload function stage 3's `POST /reload` calls — in process, not over the socket. It writes the
  lifecycle records to the event log.
- **The seam that keeps it testable.** Every verb is a pure function from the `Plan` to a list of
  actions, applied through a `Manager` trait. The real implementations are `Scm` and `Systemd`; tests
  use a recording fake, and `run`'s status reports go to a fake sink the same way. `--dry-run` on
  `install` and `uninstall` prints the actions and touches nothing. The printed list is also the
  change-management artifact the unit rule argues for.
- **What needs no admin rights.** Nothing in this goal's checks. The one step that does — installing for
  real on each platform — is a manual step the handoff names, never a check.

## Stage 6 — the response body, and `echo` into it

`crates/nvs-stdlib/src/response.rs`, `crates/nvs-stdlib/src/html.rs`,
`crates/nvs-runtime/src/ctx/output.rs`, and `nvs_server::statics` for the file body.

- **`html(Core\Html\Markup $body)`** writes the carrier's bytes as they are and declares
  `text/html; charset=utf-8`.
- **`sendFile(string $path)`.** The path is a sink (`Qual::Sink`), per
  `rule:security/response-body-is-one-typed-member`.
  - It is checked against `fs.read` at the call, and a missing path, a directory or an unreadable file
    throws there.
  - The `Ctx` then holds a declared file body, and the server answers it through the static-file policy
    — its media-type table, `Range`, and the conditional headers — streamed and never read whole.
  - `setHeader` already admits `Content-Disposition`, so there is no second parameter.
  - Striking `§15 Response::html` and `§15 Response::sendFile` from the outstanding file
    (`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:30-31`) is part of the item.
- **`echo` into an HTML response escapes**, closing `html.rs`'s and `response.rs`'s owner-M7 gaps. Under
  `OutputSink::Body` (`crates/nvs-runtime/src/ctx/output.rs:455-463`), a non-`Markup` value is escaped
  with the five-character escape `Core\Html::escape` already applies, whether it is tainted or not, and
  `Markup` is written as it is (`rule:core-classes/html-auto-escape`). The escape moves down into
  `nvs-runtime` so that it has one home, and `Core\Html::escape` calls it. Every other sink keeps the
  terminal's rendering (`rule:tooling/echo-always-has-a-sink`).

## Stage 7 — the request's input, and the test request that sends one

`crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/test.rs`, `crates/nvs-runtime/src/inproc.rs`,
and `crates/nvs-runtime/src/ctx/inbound.rs`.

- **Raw body access for an arbitrary content type is M7's, and what is missing is a whole-body `bytes`
  read.** `docs/plan/m7.md:88-92` names it for this milestone, and ADR 0024's *Revisiting* hands it to
  whoever designs `Core\Request` there. `body()` already reads every octet
  (`crates/nvs-stdlib/src/request.rs:2592`), and `bodyStream` already yields `tainted bytes` (`:1210`).
  But `body()` wraps the octets in `NvsStr::new` unchecked, so a non-UTF-8 body becomes a `string`
  that `rule:types/string-is-utf8` says cannot exist (`crates/nvs-runtime/src/string.rs:844` asserts
  it).
  - **`Core\Request::bytes(): tainted bytes`** is a buffering reader that shares `body()`'s hold under
    `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
  - **`body()` refuses an ill-formed payload** with the class `bytes as string` throws, and names
    `bytes()` in the message.
  - The spec's § 15 bullet gains the member in the same slice.
- **`Core\Test::request`** (`crates/nvs-stdlib/src/test.rs:412-428`) gains its request bag:
  `request(Http\Method $method, string $path, {headers?: array<string, string>, body?: bytes})`.
  - The query rides in the path.
  - The headers and body arrive `tainted` (`rule:testing/in-process-request`), the body through a
    production `RequestBody` over held bytes.
  - Spec § 13's cell at `docs/spec/01-core-library.md:1001` spells the whole signature.

## Stage 8 — the routes: an enum capture, the mount prefix, the per-mount origin

`crates/nvs-types/src/links.rs`, `crates/nvs-types/src/routes.rs`, `crates/nvs-stdlib/src/router.rs`,
`crates/nvs-config/src/mount.rs` and `crates/nvs-server/src/mount.rs`. Two groups share `router.rs`.

- **The enum capture**, under stage 2's rule:
  - `Core\Router::match` converts a segment to the case;
  - `closed_set` (`crates/nvs-types/src/routes.rs:1787`) answers the case spellings;
  - `within_set` (`crates/nvs-types/src/links.rs:424`) refuses a literal outside the subset at compile
    time, closing `links.rs` gap 2;
  - the generated API document emits the spellings as its `enum`.
- **The mount prefix.** When a request is being answered, `url()`'s `substitute`
  (`crates/nvs-stdlib/src/router.rs:825`) prepends the prefix `Core\Request::mount()` reports
  (`rule:routing/a-request-reads-its-mount`). Off a request, the prefix is empty and that is not an
  error. This closes `router.rs` gap 2, whose owner is the retired goal `signed-urls`.
- **The per-mount origin**, per `rule:routing/an-origin-is-per-mount-and-checked-at-boot`:
  - a served request receives its mount's resolved origin, with the captures substituted;
  - a mount whose unit makes a literal `urlAbsolute` call and resolves no origin is a boot error;
  - the check re-runs on reload.

## Stage 9 — the schedule: fleet leases, their renewal, and each entry's sub-caps

`crates/nvs-server/src/schedule.rs`, `crates/nvs-cli/src/serve.rs:753-763`, and
`crates/nvs-stdlib/src/cache/redis.rs`.

- **The lease.** The shared tier gains an internal set-if-absent: `SET key token NX PX ttl`, beside the
  existing `SET … PX` at `crates/nvs-stdlib/src/cache/redis.rs:162-197`. `nvs serve` implements
  `nvs_server::Leases` (`crates/nvs-server/src/schedule.rs:301`) over it and passes `Some` to
  `arm`, so every `scope = "fleet"` entry is armed. It is not a `Core\Cache` row: a compare-and-set a
  program could call is new cross-request coordination and not this goal
  (`rule:concurrency/cross-request-state-is-explicit`).
- **Renewal while the run is in flight** (`crates/nvs-server/src/schedule.rs:84-91`). A timer on the fire's own task
  extends the lease with one fixed, compiled-in `EVAL` that extends only while the token is still this
  host's. If the server refuses `EVAL`, the TTL alone bounds the lease (the rule's at-most-once still
  holds) and one boot note says so.
- **Each entry's `limits` and `grants`** (`crates/nvs-server/src/schedule.rs:79-82`) narrow the run's budget and
  capabilities, and never widen them. A sub-cap above the tree's own is a boot refusal.

## Stage 10 — the metrics export

`crates/nvs-server/src/metrics.rs`, `crates/nvs-server/src/route.rs`, `crates/nvs-server/src/serve.rs`,
`crates/nvs-cli/src/serve.rs`, and `crates/nvs-config/src/export.rs`.

- **Each serving core owns `Registry::of(config)`** (`crates/nvs-server/src/metrics.rs:388`), and there is
  none when the exporter is `false`. The door calls `Registry::request` with `route::label`, which
  closes `route.rs` gap 2.
- **`prometheus`.** `nvs serve` binds `[metrics] listen` at boot and answers a scrape through its own
  hyper h1 accept loop on one core. It gathers each core's series by a message over the existing
  cross-core channel, merges them arithmetically (`rule:observability/a-registry-is-per-core-and-nothing-reads-it`),
  and writes the text exposition format.
- **`otlp`** pushes the merged series on an interval through stage 2's transport.
- **Behind a cargo feature, on by default** (`rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`).
  A build without it refuses any `exporter` other than `false` at boot, naming the feature.

## Stage 11 — spans and the trace push

`crates/nvs-runtime/src/trace_context.rs`, `crates/nvs-server/src/trace.rs`, and the exporter module
stage 10 opens.

- **Exactly four kinds become a span** (`rule:observability/four-kinds-become-a-span`): the root, a
  `query`, an outbound call and a `spawn`. Each is derived from the event the timeline already files.
  A retried call is one span carrying its attempt count.
- **`[trace] sample`** decides at the root, and an inbound sampled trace is always continued.
- **A sampled request's spans** are held up to a fixed count and handed at its end to one bounded
  process-wide queue. One task pushes them over OTLP/HTTP. When the queue is full, spans are dropped and
  counted, and a collector that cannot be reached never delays a response.

## Stage 12 — the served path, end to end

`crates/nvs-cli/src/serve.rs`'s tests, beside `an_entry_that_costs_a_request` (`:1570`), and
`crates/nvs-cli/src/script.rs`'s tests.

- **A runaway under `nvs serve`.** A `while (true) {}` entry is ended by `[limits] cpu_time`, and an
  allocation loop by `[limits] memory`. Each is answered as a `FATAL`, and the same core answers the next
  request. Each piece is pinned alone today:
  - `crates/nvs-host/tests/limits.rs:309`;
  - `crates/nvs-host/src/watchdog.rs:1117`;
  - `crates/nvs-server/src/serve.rs:5928`.

  This is M6's acceptance met on the served path. If the served answer is not a `FATAL`, the fix is in
  scope.
- **The hot-reload case M7's *Verify* names that no test pins**: a revalidation that fails to compile
  fails only the requests that resolve it afterwards. Its siblings are pinned already:
  - `crates/nvs-cli/src/script.rs:970` and `:1028` — ten thousand cold requests, one compile;
  - `:1174` — a reader holding the old unit keeps answering;
  - `:1353` — a stale revalidation does not overwrite a fresher one;
  - `:1637` — revalidation is lazy and rate-capped.

  If reading `script.rs` shows an existing test that already asserts it, rename that test to the
  check's name rather than writing a second one.

## Stage 13 — the rulebook

Flip each rule below to `shipped`, with `guardedBy` filled from this goal's tests and cases, then run
`python tools/rules.py --render`:
- stage 2's two rules;
- `config/one-local-control-socket` and `config/ctl-config-reports-the-live-snapshot`;
- `packaging/a-service-is-one-stored-argv` and `packaging/a-service-answers-its-manager`;
- `http-server/a-unix-socket-listener`;
- `config/a-fleet-entry-fires-at-most-once-under-a-lease`;
- `routing/an-origin-is-per-mount-and-checked-at-boot`;
- `observability/the-exporters-are-crates` and `observability/four-kinds-become-a-span`.

## Standing decisions

- **The user's program rules, settled.**
  - M0–M8 are completed, and every promise a past milestone's plan made is built.
  - An item may be deferred to M9 or later (`docs/plan/m9.md` … `m17.md`) only if it cannot be built
    without that milestone's work — never because it is large. A deferral names that work.
  - A design call is decided under ADR 0004's ordering (security, then PHP-compatible correctness, then
    request-path latency, then simplicity, then memory) and written down: in the record where no rule
    states it, in a module doc otherwise. It is never `BLOCKED`.
  - CI is not running, so every check here runs locally. None of them needs administrator rights.
- **The control endpoint is served on one thread of its own**: a blocking accept, one connection at a
  time, so operations serialize as the rule requires. It is a thread, not a runtime
  (`rule:concurrency/one-scheduler`). It touches only `Current` and the drain bit, and it runs no Novis
  code (`rule:security/no-eval`). HTTP framing is `hyper`'s on both halves — enable its `client` feature
  for `nvs ctl` if it is absent — and never a parser of ours, for M7's own reason about FastCGI.
- **The drain has one bound.** Read `rule:concurrency/a-drain-closes-a-connection-cleanly` and the
  `[server]` keys first. If no directive bounds the drain, the stage 2 record names one, and it is not
  unbounded (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`).
- **A reload is one function.** The control socket, `systemctl reload` (by `ExecReload`) and
  `PARAMCHANGE` all end in it. A drain is one state machine: the signal, the SCM stop and the tests
  all enter `Drain::process().begin()`.
- **`nvs service` is testable through its seam, and the seam is the design.** Tests use the recording
  `Manager` and the fake status sink. Nothing in a test touches the real SCM or `systemctl`.
  - On Linux, `run` agrees with whatever `unit()` (`crates/nvs-cli/src/service.rs:491-520`) already puts
    in `ExecStart`.
  - The installer stays the `rule:packaging/the-installer-is-a-sink` front door: `plan` runs before any
    `Manager` call, so a refusal touches nothing.
- **Naming.**
  - The body reader is `Core\Request::bytes()`, named after `Core\Response::bytes`.
  - The test bag's keys are `headers` and `body`.
  - `sendFile` takes the path alone.
  - `rule:core-api/verb-lexicon` is the tie-break, and a spelling it forces instead is recorded in
    the stage 2 record.
- **An ill-formed body is refused, never repaired.** A lossy decode would silently change a webhook's
  signed payload, which spends security to buy convenience. `bytes()` is the way through.
- **The fleet lease stays inside the binary.** It is a `Leases` implementation in `nvs-cli` over
  `nvs-stdlib`'s Redis client, and `nvs-server` still names no `nvs-stdlib`
  (`crates/nvs-server/src/schedule.rs:31-43`).
- **What it spends** (`rule:programs/memory-priority`):
  - **Process-wide:**
    - one thread for the control endpoint;
    - one listener for `[metrics] listen`, and one exposition buffer per scrape in progress;
    - one bounded span queue.
  - **Per in-flight request:**
    - a sampled request's spans, up to a fixed count;
    - a `sendFile` response's open handle and one chunk buffer;
    - `bytes()`'s hold, which is `body()`'s hold under `[limits] request_body` and never a second copy.
  - **Elsewhere:** a synthetic test request's held body; one renewal timer per fleet fire in progress.

  All of it is O(in-flight) or per process, and nothing grows with requests served. The registry stays
  O(cores × series) under `max_series`.
- **ADR slots.** Stage 2's one record, and no other number.
- **Not this goal:**
  - **Owned by other goals:**
    - `Core\Metrics`'s three rows, which belong to goal `m8-stdlib-depth`;
    - the `unowned` gaps at `crates/nvs-server/src/route.rs:30` (the CSRF door),
      `crates/nvs-server/src/bounds.rs:62`, `crates/nvs-types/src/response.rs:29` and
      `crates/nvs-stdlib/src/cli.rs:130`, which goal `unowned-closures` takes after its decision sheet;
    - stale prose — `docs/plan/m7.md`'s carrier list, `crates/nvs-cli/src/serve.rs:79-90`'s "no
      configuration" gap and the "one core" rows — which belong to goal `plan-truth`.
  - **Out of scope entirely:**
    - a network control listener (`rule:config/no-network-control-surface`);
    - a TLS listener or h2c;
    - OpenRC, SysV, `rc.d` or `launchd`;
    - StatsD;
    - a program-facing compare-and-set on `Core\Cache`.

  A session that finds one of these on its path writes it to the handoff's `## Backlog`.
