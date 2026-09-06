# `rule:observability/the-runtime-exports-what-it-already-measures` — The runtime exports what it already measures, and a label may not be `tainted`

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** what the runtime exports without a line of application code — the default metric set, the spans
  derived from `rule:observability/trace-events-carry-a-kind`'s existing event kinds, and
  W3C TraceContext in both directions — plus the three-member `Core\Metrics`, the `[metrics]`/`[trace]`
  directives, and the cardinality bound. Not in scope: the coverage/trace/profile *probes* themselves, which
  are `rule:testing/debug-probes`'s and
  `rule:observability/trace-events-carry-a-kind`'s, and are reused here unchanged; and the
  sampling profiler, which stays M10's.
- **Amends:** [0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) and
  [0041](0041-timeline-export-and-gc-spawn-trace-events.md) — a fourth consumer of the same instrumentation,
  adding no probe site to the hot path, and a rule about which event kinds may become a span.
  [0051](0051-standard-library-tiers.md) § 3 — the Core roster gains `Core\Metrics`; the exporter is a
  feature-gated Native subsystem, the same split that ADR already uses for the Redis driver behind
  `Core\Cache`. [0020](0020-error-escalation-ladder.md) § 6 — the shared JSON-Lines record gains
  `trace_id`/`span_id` when a trace is active, which is what makes a log line and a span find each other.
  [0064](0064-configuration-file-format.md)/[0005](0005-config-changeability.md) — two `System`-class
  blocks. [docs/spec/01-core-library.md](../spec/01-core-library.md) § 16 — a `Core\Metrics` row.
  [docs/implementation-plan.md](../implementation-plan.md) — M7 gains the default series and the exporter,
  M8 the outbound propagation.
- **Amended by:** 0097

> **In short:** Novis already measures request duration, database query duration, GC pause and isolate spawn —
> [ADRs 0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) and
> [0041](0041-timeline-export-and-gc-spawn-trace-events.md) built the instrumentation and nothing consumes
> it in production. This exports it: a **default metric set with no code written**, spans derived from the
> **same** events (never a second set of probes), and W3C `traceparent` continued inbound and propagated
> outbound by `Core\Http\Client`. Application numbers go through a three-member **`Core\Metrics`** —
> `increment`, `observe`, `gauge`. **This does not collide with
> `rule:concurrency/cross-request-state-is-explicit`:** metrics are approximate aggregates that nothing
> reads to make a decision, so per-core accumulation with merge-at-scrape is correct, and that is exactly
> the property 0059 § 4 tests for. Two things are decided here that libraries elsewhere get wrong. **A label
> value refuses `tainted`** — unbounded cardinality is a user-supplied string reaching a label, and Novis
> already has the type that says so, so the cardinality bomb becomes a compile error. And **past the
> cardinality cap a new series is refused rather than an old one evicted**, because evicting a counter makes
> it appear to reset and silently corrupts every `rate()` over it.

## Context

- `rule:testing/debug-probes` put a debug-flags check at
  every statement and call site, and `rule:observability/trace-events-carry-a-kind` added
  `gc`, `spawn` and `query` event kinds inside routines that are already slow. Both are aimed at a
  *developer* debugging a request: Clover, Callgrind, speedscope, NDJSON. Nothing in either reaches a
  production dashboard, and the data a production dashboard needs — rate, duration, status, query time — is
  already being measured by the same code.
- **The distance between "measured" and "exported" is where every language's observability story goes
  wrong.** PHP's is: install an APM agent as a C extension, or instrument by hand with a userland client,
  or both, and reconcile the two. The agent is unsandboxed C in the request path; the userland client is a
  per-framework reimplementation. Neither has access to GC pauses or to the runtime's own scheduling, which
  are exactly the numbers that explain an unexplained p99.
- **Cardinality is the failure mode, and it is always the same failure.** A label whose value comes from the
  request — a raw path, a user id, an error message — multiplies a series into thousands, and the collector
  falls over hours later, far from the line that caused it. Every metrics library documents this and none of
  them prevent it, because in every other language a string is a string. Novis has a type that distinguishes
  *user-supplied* from *program-authored*, which turns the documentation into a diagnostic.
- **The 0059 question has to be answered explicitly or a reader will assume the worst.**
  `rule:concurrency/cross-request-state-is-explicit` forbids cross-request state and says per-core state is
  a cache, never a store, because "a program that would be incorrect if a `get` returned nothing is using
  the wrong tier". A metrics registry is per-core mutable state that outlives a request, which looks exactly
  like the thing that ADR closes. It is not, and § 5 says why in as many words.

## Decision

### 1. The default series, with no code written

Emitted by the runtime and the M7 server, present the moment an exporter is configured:

| series | kind | labels |
|---|---|---|
| `nvs_requests_total` | counter | `method`, `status`, `route` |
| `nvs_request_duration_seconds` | histogram | `method`, `status`, `route` |
| `nvs_db_query_duration_seconds` | histogram | `connection`, `operation` |
| `nvs_gc_pause_seconds` | histogram | — |
| `nvs_spawn_duration_seconds` | histogram | `kind` (`task`/`worker`/`script`) |
| `nvs_tasks_in_flight` | gauge | — |
| `nvs_deferred_trees` | gauge | — (`rule:concurrency/deferred-is-bounded-by-two-directives`) |
| `nvs_memory_bytes` | gauge | `scope` (`request`/`cache`/`process`) |
| `nvs_schedule_runs_total` | counter | `name`, `outcome` (`rule:config/scheduled-work-is-a-config-block`) |

Every one is read from instrumentation that already exists: the `query` kind from
`rule:observability/trace-events-carry-a-kind`, `gc` from § 2, `spawn` from § 3, GC and
memory from the arena accounting `rule:programs/memory-priority` already requires. **No probe site
is added to the per-statement/per-call path** `rule:testing/debug-probes`
measures and guards, and that ADR's cost claim is untouched.

**`route` is the one label that would otherwise be unbounded**, and it is why this ADR and
`rule:routing/routes-are-compiled-not-registered` interlock. It carries the **route's declared name** from 0077's
compile-time table — a closed set known at compile time — never the request's raw path. A program with no
route table simply has no `route` label; the alternative, labelling by path, is the cardinality bomb this
whole section exists to prevent, and it is not offered as an option.

### 2. Spans come from the same events, and only four kinds become one

A distributed trace is not the per-call trace `rule:testing/debug-probes`
produces, and conflating them would produce a trace with one span per function call, which no backend can
store and no human can read. So:

**Exactly four things become a span**: the request (or scheduled run) root, a `query` event, an outbound
`Core\Http\Client` call, and a `spawn` event. A `call`-kind event **never** becomes a span. `gc` becomes the
histogram in § 1, not a span — a collection pause is not a unit of work in a request's causal graph.

- An inbound **`traceparent`** header continues the trace: its trace id and parent span id are adopted, and
  its sampled flag is honoured. A malformed header **starts a new trace** rather than throwing — it arrived
  from outside, it is `tainted`, and refusing a request over a bad tracing header would make an
  observability feature into an availability one.
- **A trace id exists for every request, whatever the sampling decision.** Sampling governs whether a trace
  is *exported*, never whether an id is generated, and that is what lets the same id be Novis's only request
  identifier: `Core\Server::traceId()` reads it, every `[log]` record and every error rendering carries it,
  and it is emitted on the response so a proxy can log it with one `log_format` line. There is deliberately
  no second identifier and no inbound `X-Request-ID`
  ([0097](0097-development-server-and-proxied-origin.md) § 9).
- `Core\Http\Client` **propagates** `traceparent` outward when `[trace] propagate` is on, which is what makes
  a trace cross a service boundary at all.
- Sampling is **head-based** at the root: `[trace] sample` is the probability a *new* trace is recorded. An
  inbound trace that is already sampled is always continued, because a partially-recorded distributed trace
  is worse than none.
- **A trace id never becomes a metric label.** It is unbounded by construction, and § 4's `tainted` rule
  would refuse it anyway.

### 3. `Core\Metrics` — three members

```php
Core\Metrics::increment(string $name, {by?: uint, labels?: array<string, string>}): void;
Core\Metrics::observe(string $name, float $value, {labels?: array<string, string>}): void;
Core\Metrics::gauge(string $name, float $value, {labels?: array<string, string>}): void;
```

Subject first (`rule:core-api/shape-rules` R1), required value in dataflow order, one trailing
options shape (R2). `increment` for a counter, `observe` for a histogram, `gauge` for a point-in-time value —
three verbs for three kinds, rather than one `record` with a kind enum, because the kind is a property of
the series and not of the call, and a series recorded two ways is a bug the backend reports and the call
site cannot see.

A literal `$name` is validated at compile time against `[a-z][a-z0-9_]*`, by the same call-site literal
inspection `rule:security/secret-sinks-refuse` already performs. A name is also
fixed to one kind on first use within a process; a second kind for the same name is a runtime throw naming
both sites, because it is a mistake and not a mode.

**`Core\Metrics` is Tier 0 and always present**; whether anything leaves the process is the exporter's
question (§ 6). A binary built without the exporter feature still accumulates into the per-core registry, so
behaviour is identical across builds except for the export path — and a program that never calls
`Core\Metrics` and serves no requests holds **zero series**.

### 4. A label value refuses `tainted`

The `labels` value position is an `rule:security/tainted-qualifier` **sink**. A
`tainted string` — a query parameter, a header, a path segment, a database column — cannot become a label.

There is deliberately **no launderer for it**, because there is no sanitisation that would make it safe: the
hazard is not the value's *content*, it is that the value is drawn from an unbounded set. What exists
instead is the set of things that are already unqualified and are what a label should have been:

- an enum case or an `int` converted with `as` — `rule:security/taint-propagation`
  already launders a checked conversion, so `$statusCode as string` is a legal label for free;
- a route name from `rule:routing/routes-are-compiled-not-registered`'s closed table;
- a literal, a class constant, a configured value;
- and, when someone genuinely has a bounded user-derived set, `Core\Taint::assertTrusted` — the rare,
  greppable, reason-carrying escape hatch `rule:security/launderers-are-sink-named` already
  defines, which is exactly the right shape for a claim only the author can make.

`secret` is refused there too, and needs no new rule: a metric export is output, and
`rule:security/secret-qualifier` already refuses `secret` at every output sink.

This is the decision this ADR is most likely to be remembered for. It costs a compile error at exactly the
line that would have taken the collector down, and it is only available because Novis spent the qualifier
system on injection first.

### 5. Why this is not `rule:concurrency/cross-request-state-is-explicit`'s closed door

A per-core metrics registry is mutable state that outlives a request, which is the shape
`rule:concurrency/cross-request-state-is-explicit` exists to constrain. It passes that ADR's own test, and
the reason is worth stating rather than leaving a reader to wonder:

- **Nothing reads it to make a decision.** 0059 § 1's rule is that a program which would be *incorrect* if a
  read returned nothing is using the wrong tier. No Novis program reads a metric at all — the only reader is a
  scrape or a push, outside any request.
- **The values are approximate aggregates by design.** Per-core counters merged at scrape time are the
  correct implementation, not a compromise: it is the same reason a metric is not a rate limit
  (`rule:core-classes/ratelimit-two-members`), and the merge is arithmetic rather than coordination.
- **No request-derived value crosses**, because § 4 forbids exactly that. What accumulates is a fixed set of
  series with bounded label sets — counters and buckets, never a payload.
- **Memory is charged to the core and capped** (§ 7), the same accounting and the same exception
  `rule:concurrency/cache-memory-is-charged-to-the-core` already records for `Core\Cache::local`.

### 6. Configuration, and the tier split

```toml
[metrics]                       # System
exporter   = false              # false | "prometheus" | "otlp"
listen     = "127.0.0.1:9090"   # prometheus scrape endpoint
endpoint   = ""                 # otlp collector URL
max_series = 10000              # per core; see § 7

[trace]                         # System
exporter  = false               # false | "otlp"
endpoint  = ""
sample    = 0.01                # head-based, 0.0–1.0; an inbound sampled trace is always continued
propagate = true                # send traceparent on outbound Core\Http\Client calls
```

Both blocks are **`System`**: where a process ships telemetry, and how much it costs to do so, is a
deployment decision, and a request able to turn tracing on for itself is the reconnaissance channel
`rule:testing/debug-probes` already refuses for
`[debug] mode`. A developer wanting a trace of their own request has `Core\Debug`, which is that ADR's
surface and is unaffected.

The exporter is **Native and feature-gated** (`rule:core-api/five-placements`), defaulting
on in the server distribution, so a CLI binary or an
`rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host` single-file executable does not carry an OTLP client
and its transitive dependencies. `Core\Metrics` is Tier 0 either way — the same split 0051 § 3 already uses
for `Core\Cache`'s Redis backend, where the class is always there and the driver is a feature.

`Core\Log`'s JSON-Lines record (`rule:errors/log-write`) gains `trace_id` and
`span_id` fields whenever a trace is active. That is a two-field addition to a shape that already exists,
and it is what lets an operator jump from a log line to the trace that produced it, which is the single
highest-value thing an observability stack does.

### 7. The cardinality bound: refuse the new, keep the old

`[metrics] max_series` bounds the number of distinct label combinations a **core** holds. Past it, **a new
series is refused** — the call is a no-op — and one warning per window is written to `Core\Log` naming the
metric.

An existing series is **never evicted**, and this is a deliberate correction to the obvious design. Evicting
a counter and later recreating it makes its value appear to reset, which every backend interprets as a
process restart and which silently corrupts every `rate()` and `increase()` query over it — a wrong number
on a dashboard, which is worse than a missing one. Refusing new series loses the newest labels and keeps
every existing series exactly correct, and the warning names the metric that is producing them.

Cost, as `rule:programs/memory-priority` requires: **O(cores × series)**, bounded by
`max_series` per core, with a counter costing a few dozen bytes and a histogram its bucket array. Zero
series until something registers one. It is charged to the core, not to a request, exactly as
`rule:concurrency/cache-memory-is-charged-to-the-core`'s cache is, and it is not O(requests served).

### 8. What the crates are

`opentelemetry` and `opentelemetry-otlp` for the OTLP path including the W3C TraceContext propagator;
`metrics-exporter-prometheus` for the scrape path. All three are first-class and maintained, so
*Decisions taken at project start*'s standing rule applies straightforwardly: this is somebody else's
specification and it is a dependency. What is ours is the wiring from
`rule:observability/trace-events-carry-a-kind`'s event kinds to spans, and the per-core
registry, both of which are about Novis's own runtime and could not be a crate.

## Consequences

**Positive**

- **The dashboard exists before the application does.** Request rate, latency and error rate, plus database
  time and GC pause, with one directive and no code — and the two that no APM agent can give (GC pause,
  isolate spawn overhead) are the ones that explain an unexplained p99.
- **One instrumentation, four consumers.** Coverage, the speedscope timeline, the deterministic profiler and
  now production telemetry all read the same events, so a number cannot disagree with itself depending on
  which tool asked.
- **The cardinality bomb is a compile error.** Every other language documents this failure; Novis had already
  built the type that prevents it, and § 4 is the whole of the work.
- **Logs and traces correlate**, because both go through one record shape
  (`rule:errors/log-write`) and it now carries the ids.
- **A wrong dashboard number is preferred against.** § 7's refuse-don't-evict choice keeps every reported
  series exactly correct, which is the property a dashboard is for.
- **No unsandboxed C agent in the request path**, which is what the PHP alternative looks like.

**Negative**

- **`tainted` labels will be hit, and the first reaction will be annoyance.** Labelling by user id, tenant
  name or error message is what people do. The diagnostic has to name the alternatives (§ 4) rather than
  just refusing, and `Core\Taint::assertTrusted` has to be discoverable from it.
- **`route` depends on `rule:routing/routes-are-compiled-not-registered`.** An application that does not use Novis's
  route table gets no `route` label at all, and per-endpoint latency is the second thing anyone looks at.
  The alternative was a cardinality bomb, so this is the right refusal, but it is a real coupling between
  two otherwise independent features.
- **Head sampling loses the interesting traces.** A 1% head sample records 1% of the errors too, and tail
  sampling — decide after the fact, keep the slow and failed ones — is what people actually want. It needs a
  collector-side component or a buffering exporter, and it is named in *Revisiting* rather than built.
- **Two more `System` blocks** (priority 4), and an exporter's dependency tree in the default server binary.
  Feature-gated, so a CLI build does not pay it, but the server build does.
- **Refusing new series past the cap is a silent-ish loss.** One warning per window is easy to miss, and the
  labels that go missing are the newest ones, which are often the ones being debugged. It is still better
  than a corrupted `rate()`.
- **`Core\Metrics` accumulates even with no exporter built**, so a CLI program calling it holds a registry
  nothing will ever read. Bounded and tiny, and the alternative — the members being no-ops in one build and
  not in another — is a difference between builds, which is worse.

## Alternatives rejected

- **Leave it to an extension or userland.** The PHP answer. Rejected: the numbers that matter most (GC,
  spawn, arena memory) are inside the runtime and unreachable from outside it, and the two userland options
  are an unsandboxed C agent or a per-framework reimplementation.
- **Emit a span per `call` event.** Complete, and reuses the existing instrumentation with no filtering.
  Rejected in § 2: a trace with one span per function call is unstorable and unreadable, and it would put
  export cost on the hot path `rule:testing/debug-probes`
  deliberately keeps cheap.
- **Label by request path rather than route name.** Zero coupling to
  `rule:routing/routes-are-compiled-not-registered`. Rejected: `/users/1`, `/users/2`, … is one series per user, which
  is the exact failure this ADR is built to prevent — and offering it as an option means it will be chosen.
- **Allow `tainted` labels with a runtime cardinality guard.** More permissive, and the guard catches it
  eventually. Rejected: "eventually" is after the collector has the series, and a compile error at the line
  responsible is strictly better than a warning hours later on a different machine.
- **Evict the least-recently-used series past the cap**, as the queue's original sketch assumed. Rejected in
  § 7: a recreated counter reads as a reset and corrupts every rate query over it. A wrong number on a
  dashboard is worse than a missing one.
- **A shared, coherent metrics store instead of per-core registries.** Rejected: it is exactly the
  cross-request coordination `rule:concurrency/cross-request-state-is-explicit` closes, for a value that is
  approximate by definition and is merged at scrape anyway.
- **`Core\Metrics` at Tier 2 alongside its exporter.** Smaller Tier 0. Rejected: a program's instrumentation
  calls would then compile in one build and not another, which makes the `Core` namespace conditional —
  precisely what `rule:core-api/core-means-always-present` forbids.
- **One `Metrics::record(name, value, Kind)` member** instead of three. Rejected: the kind is a property of
  the series, not the call (`rule:core-api/shape-rules` R11's instinct), and one name recorded
  as two kinds is a bug the call site cannot see.
- **StatsD as the wire format.** Simplest possible exporter. Rejected: no histogram semantics worth the
  name, no trace story at all, and OTLP plus Prometheus scrape covers essentially every deployment.

## Revisiting

- **Tail sampling** — keeping traces that were slow or failed rather than a fixed head fraction — is what
  operators actually want and is deliberately not built. It needs either a collector-side component (which is
  configuration, not Novis's code) or a buffering exporter that holds every span until a request ends (which is
  a footprint decision under `rule:programs/memory-priority` and needs its own number).
- **Exemplars** — attaching a trace id to a histogram bucket, so a slow-request bucket links to a trace — are
  the highest-value thing missing here and are additive.
- **A per-attempt span for a retried outbound call**, flagged by
  `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s *Revisiting*: one span with an attempt count, or one
  per attempt. One span with a count is the current behaviour by default; the question is whether a retried
  call's individual latencies are worth the span multiplication.
- **A `Core\Metrics` read path**, if anything ever needs to read its own counters. It would land squarely in
  `rule:concurrency/cross-request-state-is-explicit`'s territory and would need § 5's argument re-made,
  because a program that reads a metric to decide something is using the wrong tier.
- **Whether `nvs_db_query_duration_seconds` should carry a statement label** — normalised SQL rather than
  just `operation`. Valuable, and a cardinality question that needs a bound before it is offered.

## Verification

- **M7:** a server with `[metrics] exporter = "prometheus"` and no application code exposes every § 1 series
  on the scrape endpoint, with correct counts under a load fixture; with `exporter = false` the endpoint does
  not exist.
- **M7:** a guard test asserts the per-statement/per-call debug-flag cost from
  `rule:testing/debug-probes`'s own *Revisiting* shows no
  regression attributable to this ADR, since no site was added to that path.
- **M7:** an inbound `traceparent` is continued (same trace id, the root span's parent set from it); a
  malformed one starts a new trace and does not fail the request; a `traceparent` from an untrusted client
  never reaches a metric label.
- **M7:** `route` carries `rule:routing/routes-are-compiled-not-registered`'s declared name, and a program with no
  route table emits the series without the label rather than with a path.
- **M7:** exceeding `max_series` refuses the new series, leaves every existing series' value unchanged
  across the boundary — the assertion that distinguishes this from eviction — and writes exactly one warning
  per window.
- **M4S/M8:** a `tainted string` in a `labels` value is a compile-time diagnostic naming § 4 and the
  alternatives; `$status as string` from an `int` compiles; `Core\Taint::assertTrusted` compiles; a `secret`
  is refused naming `rule:security/secret-qualifier`.
- **M8:** `Core\Metrics::increment` on a name previously used as a gauge throws naming both call sites; a
  name that is not `[a-z][a-z0-9_]*` is a compile error at a literal call site.
- **M8:** `Core\Http\Client` sends `traceparent` when `[trace] propagate` is on and omits it when off; a
  `query`, an outbound call and a `spawn` each produce exactly one span, and a `call` event produces none.
- **M8:** a `Core\Log::write` inside a sampled request carries `trace_id`/`span_id` in the same JSON-Lines
  record `rule:errors/log-write` defines, and outside one omits both fields rather
  than emitting empty strings.
- **M8:** a build with the exporter feature off still accumulates into the registry and still compiles every
  `Core\Metrics` call — the assertion that keeps the two builds' behaviour identical.
