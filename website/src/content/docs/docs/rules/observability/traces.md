---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Traces and spans"
description: "Exactly four things become a span, an inbound traceparent is continued, and sampling is decided once at the root."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/observability/metrics/
  label: "Metrics"
next:
  link: /docs/rules/observability/exit-hooks/
  label: "Exit hooks"
---

<p class="nv-section-lead">Exactly four things become a span, an inbound <code>traceparent</code> is continued, and sampling is decided once at the root.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">16</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">7</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">9</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">10</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#four-kinds-become-a-span">Exactly four things become a span, and a <code>call</code> event never does</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-call-never-becomes-a-span">A <code>call</code> event never becomes a span; the root, a <code>query</code>, an outbound HTTP call and a <code>spawn</code> do, and <code>gc</code> becomes a metric</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#trace-events-carry-a-kind">A trace event carries one of five kinds — <code>call</code>, <code>gc</code>, <code>spawn</code>, <code>query</code>, <code>http</code> — and a <code>call</code> keeps its probe shape</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-query-is-a-trace-event">A statement is a <code>query</code> trace event that carries its duration, driver, connection, truncated SQL and row counts, and never a bound parameter</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-slow-query-is-logged-past-a-threshold">A <code>[db.&lt;name&gt;] slow_query</code> threshold writes a statement's span facts to <code>Core\Log</code>, and is off until a block writes one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#gc-pause-is-its-own-event">A collector pause is a <code>gc</code> event recorded from the collector's run routine, never from the safepoint poll</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#spawn-is-its-own-event">An isolate spawn and its join are one <code>spawn</code> event with an overhead split, and the child's stream is stitched in at export time</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#an-inbound-traceparent-is-continued">An inbound <code>traceparent</code> is continued, and one the process cannot read starts a new trace rather than failing the request</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#an-outbound-call-propagates-traceparent"><code>Core\Http\Client</code> sends <code>traceparent</code> while <code>[trace] propagate</code> is on, and the id it sends is the request's own</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#sampling-is-head-based">Sampling is decided once at the root by <code>[trace] sample</code>, and an inbound trace that is already sampled is always continued</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-exporter-is-a-feature-and-core-metrics-is-not">The exporter is a feature-gated Native subsystem, and <code>Core\Metrics</code> is Tier 0 in every build</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-exporters-are-crates">The OTLP and Prometheus paths are dependencies; the event-to-span wiring and the per-core registry are ours</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#speedscope-timeline-export">The four event kinds export as one speedscope evented timeline, beside Callgrind, Clover/lcov and NDJSON</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#memory-is-three-numbers-on-core-budget">A request reads its own memory as <code>memoryHeld</code>, <code>memoryPeak</code> and <code>memoryLimit</code> on <code>Core\Budget</code>, and the process's resident bytes are <code>Core\Os</code>'s</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-memory-peak-is-recorded-not-asked-for">The runtime records a request's memory high-water mark in every build, and a nested isolate restores the enclosing mark rather than clobbering it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#memory-high-water-writes-a-warn">A <code>[limits] memory_high_water</code> fraction writes one <code>Warn</code> when a request's memory peak crosses that share of its ceiling, and is off until a block writes one</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="four-kinds-become-a-span">

## Exactly four things become a span, and a `call` event never does

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#four-kinds-become-a-span"><code>observability/four-kinds-become-a-span</code></a>
</div>

A distributed trace is not the per-call trace [`testing/debug-probes`](/docs/rules/testing/coverage-and-probes/#debug-probes "Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier") produces, and conflating
them would give a trace one span per function call, which no backend can store and no human can
read. So **exactly four things become a span**: the request (or scheduled run) root, a `query`
event ([`observability/a-query-is-a-trace-event`](/docs/rules/observability/traces/#a-query-is-a-trace-event "A statement is a query trace event that carries its duration, driver, connection, truncated SQL and row counts, and never a bound parameter")), an outbound `Core\Http\Client` call, and a
`spawn` event. A `call`-kind event **never** becomes a span. A `gc` event becomes the pause
histogram in [`observability/default-series`](/docs/rules/observability/metrics/#default-series "Ten series exist the moment an exporter is configured, with no application code written"), not a span — a collection pause is not a unit of
work in a request's causal graph.

The spans are derived from the same events the timeline already files; there is no second set of
probes for tracing, and export cost stays off the hot path the debug probes deliberately keep cheap.
A retried outbound call is one span carrying an attempt count, not one span per attempt.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A distributed trace is not Xdebug's per-call function trace; a span is a request, a query, an outbound call or a spawn, and nothing finer</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#a-query-is-a-trace-event" title="A statement is a query trace event that carries its duration, driver, connection, truncated SQL and row counts, and never a bound parameter"><code>observability/a-query-is-a-trace-event</code></a> <a href="/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request" title="A trace id exists for every request whatever the sampling decision, and it is the only request identifier"><code>observability/a-trace-id-exists-for-every-request</code></a> <a href="/docs/rules/observability/traces/#sampling-is-head-based" title="Sampling is decided once at the root by [trace] sample, and an inbound trace that is already sampled is always continued"><code>observability/sampling-is-head-based</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/observability/traces/#trace-events-carry-a-kind" title="A trace event carries one of five kinds — call, gc, spawn, query, http — and a call keeps its probe shape"><code>observability/trace-events-carry-a-kind</code></a> <a href="/docs/rules/observability/traces/#gc-pause-is-its-own-event" title="A collector pause is a gc event recorded from the collector's run routine, never from the safepoint poll"><code>observability/gc-pause-is-its-own-event</code></a> <a href="/docs/rules/observability/traces/#spawn-is-its-own-event" title="An isolate spawn and its join are one spawn event with an overhead split, and the child's stream is stitched in at export time"><code>observability/spawn-is-its-own-event</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0041.md">record 0041</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-call-never-becomes-a-span">

## A `call` event never becomes a span; the root, a `query`, an outbound HTTP call and a `spawn` do, and `gc` becomes a metric

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-call-never-becomes-a-span"><code>observability/a-call-never-becomes-a-span</code></a>
</div>

Production telemetry reads the same four event kinds the trace records and adds no instrumentation
of its own — and it is bound by one rule the taxonomy owns: **a `call`-kind event never becomes a
distributed-tracing span.** Exactly four things do: the request or scheduled-run root, a `query`, an
outbound HTTP call, and a `spawn`. A `gc` event becomes a metric, not a span, because a collection
pause is not a unit of work in a request's causal graph.

The set is closed for two reasons. A trace with one span per function call is unstorable by any
backend. And admitting one would put export cost on the per-call path that
[`testing/debug-probes`](/docs/rules/testing/coverage-and-probes/#debug-probes "Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier") keeps to a load and a predicted-not-taken branch — the cost class every
other kind was placed off of on purpose ([`observability/gc-pause-is-its-own-event`](/docs/rules/observability/traces/#gc-pause-is-its-own-event "A collector pause is a gc event recorded from the collector's run routine, never from the safepoint poll"),
[`observability/spawn-is-its-own-event`](/docs/rules/observability/traces/#spawn-is-its-own-event "An isolate spawn and its join are one spawn event with an overhead split, and the child's stream is stitched in at export time")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>An OpenTelemetry SDK installed as a library lets a program open a span around any function; here the set of things that become a span is closed and a function call is not in it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#trace-events-carry-a-kind" title="A trace event carries one of five kinds — call, gc, spawn, query, http — and a call keeps its probe shape"><code>observability/trace-events-carry-a-kind</code></a> <a href="/docs/rules/observability/traces/#gc-pause-is-its-own-event" title="A collector pause is a gc event recorded from the collector's run routine, never from the safepoint poll"><code>observability/gc-pause-is-its-own-event</code></a> <a href="/docs/rules/observability/traces/#spawn-is-its-own-event" title="An isolate spawn and its join are one spawn event with an overhead split, and the child's stream is stitched in at export time"><code>observability/spawn-is-its-own-event</code></a> <a href="/docs/rules/observability/metrics/#default-series" title="Ten series exist the moment an exporter is configured, with no application code written"><code>observability/default-series</code></a> <a href="/docs/rules/observability/metrics/#a-registry-is-per-core-and-nothing-reads-it" title="A metrics registry is per core, merged at scrape, and no program reads a metric"><code>observability/a-registry-is-per-core-and-nothing-reads-it</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0041.md">record 0041</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a></dd></div></dl>

</div>

<div class="nv-rule" id="trace-events-carry-a-kind">

## A trace event carries one of five kinds — `call`, `gc`, `spawn`, `query`, `http` — and a `call` keeps its probe shape

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#trace-events-carry-a-kind"><code>observability/trace-events-carry-a-kind</code></a>
</div>

Every trace event carries a `kind` tag, and the tag is one of exactly five: `call`, `gc`, `spawn`,
`query`, `http`. A `call` event is [`testing/debug-probes`](/docs/rules/testing/coverage-and-probes/#debug-probes "Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier")'s probe pair unchanged — callee,
arguments, entry and exit timestamp, checked-return status, result. The other kinds are emitted from
routines of their own, not from the per-statement or per-call probe: a `gc` from the collector's run
routine ([`observability/gc-pause-is-its-own-event`](/docs/rules/observability/traces/#gc-pause-is-its-own-event "A collector pause is a gc event recorded from the collector's run routine, never from the safepoint poll")), a `spawn` from the three isolate-spawn
routines ([`observability/spawn-is-its-own-event`](/docs/rules/observability/traces/#spawn-is-its-own-event "An isolate spawn and its join are one spawn event with an overhead split, and the child's stream is stitched in at export time")), a `query` from inside `Core\Db`'s own
statement routine, and an `http` from `Core\Http\Client`'s transport, filed once per call whatever
its attempt count.

A `query` carries the duration, the driver, the connection name, the truncated SQL text, the rows
returned and the rows affected — and **never a bound parameter value**, because a trace is a
`secret` sink ([`security/secret-qualifier`](/docs/rules/security/secrets/#secret-qualifier "secret is a second, independent compile-time qualifier, written before tainted and in that order alone")). An `http` event answers the same question about an
outbound call: the method, the scheme, host and port, the path without its query, the status, the
attempt and hop counts, the address connected to, and where the time went — and never a query
string, a header value or a body, for the same reason. Every one of those kinds sits in a routine
that is already rare and already slow, so tracing costs nothing on the hot path the probes measure,
and nothing at all when the flag is off.

The tag is what lets one stream serve every consumer: the timeline export, the profiler's
attribution, and production telemetry all read these kinds and add no instrumentation of their
own.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Xdebug's trace is a log of function calls; here a collector pause, an isolate spawn and a database statement are events of their own on the same stream, tagged so a viewer can tell them apart</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/observability/traces/#a-query-is-a-trace-event" title="A statement is a query trace event that carries its duration, driver, connection, truncated SQL and row counts, and never a bound parameter"><code>observability/a-query-is-a-trace-event</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0041.md">record 0041</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/src/ctx/trace.rs"><code>crates/nvs-runtime/src/ctx/trace.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-query-is-a-trace-event">

## A statement is a `query` trace event that carries its duration, driver, connection, truncated SQL and row counts, and never a bound parameter

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-query-is-a-trace-event"><code>observability/a-query-is-a-trace-event</code></a>
</div>

A statement is a `query` trace event, beside `call`, `gc` and `spawn`, instrumented in the driver's
own statement routine and off the hot path exactly as those two are. The event carries **duration,
driver, connection name, truncated SQL text, rows returned and rows affected — and never a bound
parameter.** The span type is handed the SQL and the clock and is never handed the values, so there
is no parameter in scope for a later field, a later rendering or a later driver to leak; the SQL is
safe to carry because no driver ever interpolates a value into it, so what the span holds is the
statement as written, placeholders still placeholders. [`core-classes/db-error`](/docs/rules/core-classes/running-a-statement/#db-error "One DbError carries a normalised kind across every driver, and never a bound parameter") states the same
rule for the error path.

The driver is a field so the five backends contribute one event and a trace reads across them. The
duration is measured from the moment the statement went out, not from its first row, so a span
reports what the caller waited. The same span feeds
[`observability/a-slow-query-is-logged-past-a-threshold`](/docs/rules/observability/traces/#a-slow-query-is-logged-past-a-threshold "A [db.<name>] slow_query threshold writes a statement's span facts to Core\Log, and is off until a block writes one"), rendered at most once however many
readers there are, and it is what becomes a query span in
[`observability/four-kinds-become-a-span`](/docs/rules/observability/traces/#four-kinds-become-a-span "Exactly four things become a span, and a call event never does"). Both outputs are inert unless asked for.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There are no mysqlnd statistics and no general query log; a statement is a trace event, filed in the driver and off the hot path, and it never carries a bound value</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#a-slow-query-is-logged-past-a-threshold" title="A [db.&lt;name&gt;] slow_query threshold writes a statement's span facts to Core\Log, and is off until a block writes one"><code>observability/a-slow-query-is-logged-past-a-threshold</code></a> <a href="/docs/rules/observability/traces/#four-kinds-become-a-span" title="Exactly four things become a span, and a call event never does"><code>observability/four-kinds-become-a-span</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-error" title="One DbError carries a normalised kind across every driver, and never a bound parameter"><code>core-classes/db-error</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-is-named" title="A connection is named in configuration or built from settings, and both memoize for the request"><code>core-classes/db-connection-is-named</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/observability/traces/#trace-events-carry-a-kind" title="A trace event carries one of five kinds — call, gc, spawn, query, http — and a call keeps its probe shape"><code>observability/trace-events-carry-a-kind</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0041.md">record 0041</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/src/ctx/trace.rs"><code>crates/nvs-runtime/src/ctx/trace.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/src/span.rs"><code>crates/nvs-db/src/span.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-slow-query-is-logged-past-a-threshold">

## A `[db.<name>] slow_query` threshold writes a statement's span facts to `Core\Log`, and is off until a block writes one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-slow-query-is-logged-past-a-threshold"><code>observability/a-slow-query-is-logged-past-a-threshold</code></a>
</div>

A `[db.<name>]` block may write `slow_query`, a duration such as `"200ms"` or `"1s"`; a statement on
that connection that runs longer writes the same facts its trace event carries — duration, driver,
connection, truncated SQL, row counts, never a parameter — to `Core\Log`
([`observability/a-query-is-a-trace-event`](/docs/rules/observability/traces/#a-query-is-a-trace-event "A statement is a query trace event that carries its duration, driver, connection, truncated SQL and row counts, and never a bound parameter")).

**Unwritten is off**, and off is silent: a deployment gets no slow-query log it did not ask for. A
written `0` is a threshold every statement passes, not a second spelling of off. A connection with
no block at all — one opened by the program with settings no operator named
([`core-classes/db-connection-is-named`](/docs/rules/core-classes/connecting-to-a-database/#db-connection-is-named "A connection is named in configuration or built from settings, and both memoize for the request")) — has no threshold either. A value that is not a
duration is refused at boot, where it is written.

The threshold and the trace are asked once, *before* a statement borrows the context, so a request
that turns either on midway through a statement gets a whole event or none, never half of one.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The slow-query log is a per-connection threshold in <code>nvs.toml</code> written by the application's operator, not the server's <code>slow_query_log</code>, and it reports the same facts the trace does</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#a-query-is-a-trace-event" title="A statement is a query trace event that carries its duration, driver, connection, truncated SQL and row counts, and never a bound parameter"><code>observability/a-query-is-a-trace-event</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-is-named" title="A connection is named in configuration or built from settings, and both memoize for the request"><code>core-classes/db-connection-is-named</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-write" title="One write path, and the engine floor is its other caller"><code>errors/log-write</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/db.rs"><code>crates/nvs-config/tests/db.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="gc-pause-is-its-own-event">

## A collector pause is a `gc` event recorded from the collector's run routine, never from the safepoint poll

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#gc-pause-is-its-own-event"><code>observability/gc-pause-is-its-own-event</code></a>
</div>

A stop-the-world collection run is a trace event of its own, `kind: gc`, recorded from **inside the
cycle collector's run routine** and gated by the same `TRACE`/`PROFILE` bits `Ctx` already carries.
It records a start timestamp, a duration and the number of objects freed.

It is emitted from the collector's routine and never from the safepoint poll, because the poll is
checked on every loop back-edge and function entry — precisely the hot path tracing must not touch —
while the collection run is already the rare, slow path where one more branch and a timestamp pair
cost nothing relative to the run itself.

What the event buys is honest attribution. Without it a pause is silently folded into the self time
of whichever function happened to be executing, and `PROFILE`'s self/inclusive figures carry a cost
that function did not cause. With it the pause is its own bar on the timeline, and in production
telemetry it becomes a metric rather than a span ([`observability/a-call-never-becomes-a-span`](/docs/rules/observability/traces/#a-call-never-becomes-a-span "A call event never becomes a span; the root, a query, an outbound HTTP call and a spawn do, and gc becomes a metric")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#trace-events-carry-a-kind" title="A trace event carries one of five kinds — call, gc, spawn, query, http — and a call keeps its probe shape"><code>observability/trace-events-carry-a-kind</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/observability/metrics/#a-registry-is-per-core-and-nothing-reads-it" title="A metrics registry is per core, merged at scrape, and no program reads a metric"><code>observability/a-registry-is-per-core-and-nothing-reads-it</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0041.md">record 0041</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a></dd></div></dl>

</div>

<div class="nv-rule" id="spawn-is-its-own-event">

## An isolate spawn and its join are one `spawn` event with an overhead split, and the child's stream is stitched in at export time

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#spawn-is-its-own-event"><code>observability/spawn-is-its-own-event</code></a>
</div>

Each of the three spawn constructs — `spawn`, `spawn worker`, `spawn script` — emits a `kind: spawn`
event from its own runtime routine, and its join or result point closes it, gated by the same
`TRACE`/`PROFILE` bits. The event records the start timestamp, which of the three forms it was, the
join timestamp, and a computed overhead split: the parent-observed wall time minus the child-reported
wall time that already arrives on `ScriptResult`. That gives "real child compute" and "isolate
scheduling and copy-out cost" as two numbers instead of one opaque total.

A spawn has its own instrumentation point because it is not a function call and does not flow
through the call probe — [`security/isolate-shares-nothing`](/docs/rules/security/isolates/#isolate-shares-nothing "Running another script is an in-process isolate that shares nothing with its parent but compiled code") rejects modelling it as one.

The child's own trace or profile stream is **never merged live** into the parent's during the spawn;
that would cross the arena boundary isolation exists to keep. Showing a child's events nested under
its parent's `spawn` bar, anchored at the spawn's timestamp, is an export-time operation in the CLI's
exporter over data that already crosses the boundary on `ScriptResult` — not a new live cross-arena
mechanism.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#trace-events-carry-a-kind" title="A trace event carries one of five kinds — call, gc, spawn, query, http — and a call keeps its probe shape"><code>observability/trace-events-carry-a-kind</code></a> <a href="/docs/rules/security/isolates/#isolate-shares-nothing" title="Running another script is an in-process isolate that shares nothing with its parent but compiled code"><code>security/isolate-shares-nothing</code></a> <a href="/docs/rules/security/isolates/#isolate-failure-is-a-value" title="A child isolate's failure arrives as data on the result and never unwinds into its parent"><code>security/isolate-failure-is-a-value</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0041.md">record 0041</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a></dd></div></dl>

</div>

<div class="nv-rule" id="an-inbound-traceparent-is-continued">

## An inbound `traceparent` is continued, and one the process cannot read starts a new trace rather than failing the request

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#an-inbound-traceparent-is-continued"><code>observability/an-inbound-traceparent-is-continued</code></a>
</div>

An inbound W3C `traceparent` header continues the trace: its trace id and parent span id are
adopted, and its sampled flag is honoured. A request that arrived without one is a new trace rather
than no trace ([`observability/a-trace-id-exists-for-every-request`](/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request "A trace id exists for every request whatever the sampling decision, and it is the only request identifier")).

A header the process cannot read — malformed, or two `traceparent` lines on one request, for which
the W3C format has no combining rule — **starts a new trace rather than throwing**. It arrived from
outside, it is `tainted`, and refusing a request over a bad tracing header would turn an
observability feature into an availability one.

An inbound trace that is already sampled is always continued, because a partially recorded
distributed trace is worse than none; that half of the rule is
[`observability/sampling-is-head-based`](/docs/rules/observability/traces/#sampling-is-head-based "Sampling is decided once at the root by [trace] sample, and an inbound trace that is already sampled is always continued")'s. The other direction is
[`observability/an-outbound-call-propagates-traceparent`](/docs/rules/observability/traces/#an-outbound-call-propagates-traceparent "Core\Http\Client sends traceparent while [trace] propagate is on, and the id it sends is the request's own").

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request" title="A trace id exists for every request whatever the sampling decision, and it is the only request identifier"><code>observability/a-trace-id-exists-for-every-request</code></a> <a href="/docs/rules/observability/traces/#an-outbound-call-propagates-traceparent" title="Core\Http\Client sends traceparent while [trace] propagate is on, and the id it sends is the request's own"><code>observability/an-outbound-call-propagates-traceparent</code></a> <a href="/docs/rules/observability/traces/#sampling-is-head-based" title="Sampling is decided once at the root by [trace] sample, and an inbound trace that is already sampled is always continued"><code>observability/sampling-is-head-based</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/trace.rs"><code>crates/nvs-server/src/trace.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/src/trace_context.rs"><code>crates/nvs-runtime/src/trace_context.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-outbound-call-propagates-traceparent">

## `Core\Http\Client` sends `traceparent` while `[trace] propagate` is on, and the id it sends is the request's own

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-outbound-call-propagates-traceparent"><code>observability/an-outbound-call-propagates-traceparent</code></a>
</div>

`Core\Http\Client` sends `traceparent` on every outbound call while `[trace] propagate` is on, which
is what makes a trace cross a service boundary at all. `propagate` ships **on**
([`observability/metrics-and-trace-blocks-are-system`](/docs/rules/observability/metrics/#metrics-and-trace-blocks-are-system "[metrics] and [trace] are System blocks, and a value neither exporter accepts is refused at boot")); off, no header leaves.

What leaves is the runtime's own id — one per request, from
[`observability/a-trace-id-exists-for-every-request`](/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request "A trace id exists for every request whatever the sampling decision, and it is the only request identifier") — never one minted per call. Exactly one
`traceparent` is sent: a caller that wrote its own header keeps it, because a second line beside it
is what the W3C format tells a receiver to read as no header at all, and that would end the trace at
this hop while looking right on the line that sent ours.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A trace crosses a service boundary with no header code in the application, where a PHP client has to be wrapped to carry one</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request" title="A trace id exists for every request whatever the sampling decision, and it is the only request identifier"><code>observability/a-trace-id-exists-for-every-request</code></a> <a href="/docs/rules/observability/traces/#an-inbound-traceparent-is-continued" title="An inbound traceparent is continued, and one the process cannot read starts a new trace rather than failing the request"><code>observability/an-inbound-traceparent-is-continued</code></a> <a href="/docs/rules/observability/metrics/#metrics-and-trace-blocks-are-system" title="[metrics] and [trace] are System blocks, and a value neither exporter accepts is refused at boot"><code>observability/metrics-and-trace-blocks-are-system</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http.rs"><code>crates/nvs-stdlib/src/http.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="sampling-is-head-based">

## Sampling is decided once at the root by `[trace] sample`, and an inbound trace that is already sampled is always continued

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#sampling-is-head-based"><code>observability/sampling-is-head-based</code></a>
</div>

Sampling is **head-based at the root**: `[trace] sample` is the probability, from `0.0` to `1.0`,
that a *new* trace is recorded. An inbound trace that is already sampled is always continued
regardless of the local fraction, because a partially recorded distributed trace is worse than none.

The decision governs export only. An id exists for every request whether or not it is sampled
([`observability/a-trace-id-exists-for-every-request`](/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request "A trace id exists for every request whatever the sampling decision, and it is the only request identifier")), so an unsampled request still has a log
line that can be correlated with a proxy's.

Head sampling records one per cent of the errors too, when one per cent is the fraction. Tail
sampling — decide after the fact, keep the slow and the failed — needs a collector-side component or
a buffering exporter, and is deliberately not built.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request" title="A trace id exists for every request whatever the sampling decision, and it is the only request identifier"><code>observability/a-trace-id-exists-for-every-request</code></a> <a href="/docs/rules/observability/traces/#an-inbound-traceparent-is-continued" title="An inbound traceparent is continued, and one the process cannot read starts a new trace rather than failing the request"><code>observability/an-inbound-traceparent-is-continued</code></a> <a href="/docs/rules/observability/metrics/#metrics-and-trace-blocks-are-system" title="[metrics] and [trace] are System blocks, and a value neither exporter accepts is refused at boot"><code>observability/metrics-and-trace-blocks-are-system</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-exporter-is-a-feature-and-core-metrics-is-not">

## The exporter is a feature-gated Native subsystem, and `Core\Metrics` is Tier 0 in every build

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-exporter-is-a-feature-and-core-metrics-is-not"><code>observability/the-exporter-is-a-feature-and-core-metrics-is-not</code></a>
</div>

The exporter — the Prometheus scrape endpoint and the OTLP client — is a **Native, feature-gated**
subsystem ([`core-api/five-placements`](/docs/rules/core-api/what-belongs-in-core/#five-placements "A candidate lands in exactly one of five placements: Core, Native, Ext, Dropped or answered")), on by default in the server distribution, so a CLI
binary or a single-file executable does not carry an OTLP client and its transitive dependencies.

`Core\Metrics` is **Tier 0 in every build**. Placing it beside its exporter would make a program's
instrumentation calls compile in one build and not another, which makes the `Core` namespace
conditional — precisely what [`core-api/core-means-always-present`](/docs/rules/core-api/what-belongs-in-core/#core-means-always-present "Core means always present, so nothing outside Tier 0 may register a name under it") forbids. So a build without
the exporter still compiles every `Core\Metrics` call and still accumulates into the registry; a CLI
program that calls it holds a registry nothing will ever read, bounded and tiny, and the alternative
— members that are no-ops in one build and not in another — is a difference between builds, which
is worse. It is the same split the Core roster already uses for `Core\Cache`'s Redis backend, where
the class is always there and the driver is a feature.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/what-belongs-in-core/#five-placements" title="A candidate lands in exactly one of five placements: Core, Native, Ext, Dropped or answered"><code>core-api/five-placements</code></a> <a href="/docs/rules/core-api/what-belongs-in-core/#core-means-always-present" title="Core means always present, so nothing outside Tier 0 may register a name under it"><code>core-api/core-means-always-present</code></a> <a href="/docs/rules/core-api/what-belongs-in-core/#tier-placement" title="A library candidate is placed by six ordered tests, not by PHP's extension list"><code>core-api/tier-placement</code></a> <a href="/docs/rules/observability/metrics/#metrics-three-members" title="Core\Metrics is three verbs for three kinds, always present, and accumulates whether or not an exporter is built"><code>observability/metrics-three-members</code></a> <a href="/docs/rules/observability/traces/#the-exporters-are-crates" title="The OTLP and Prometheus paths are dependencies; the event-to-span wiring and the per-core registry are ours"><code>observability/the-exporters-are-crates</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0048.md">record 0048</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-exporters-are-crates">

## The OTLP and Prometheus paths are dependencies; the event-to-span wiring and the per-core registry are ours

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-exporters-are-crates"><code>observability/the-exporters-are-crates</code></a>
</div>

The OTLP path, including the W3C TraceContext propagator, and the Prometheus scrape path are
somebody else's specification and are taken as dependencies — `opentelemetry` and
`opentelemetry-otlp` for the one, `metrics-exporter-prometheus` for the other — behind the feature
[`observability/the-exporter-is-a-feature-and-core-metrics-is-not`](/docs/rules/observability/traces/#the-exporter-is-a-feature-and-core-metrics-is-not "The exporter is a feature-gated Native subsystem, and Core\Metrics is Tier 0 in every build") describes.

What is ours, and could not be a crate, is the wiring from the runtime's own event kinds to spans
([`observability/four-kinds-become-a-span`](/docs/rules/observability/traces/#four-kinds-become-a-span "Exactly four things become a span, and a call event never does")) and the per-core registry
([`observability/a-registry-is-per-core-and-nothing-reads-it`](/docs/rules/observability/metrics/#a-registry-is-per-core-and-nothing-reads-it "A metrics registry is per core, merged at scrape, and no program reads a metric")). The registry is deliberately
whole without an exporter: an exporter reads its series in order and formats them, and adding one
changes nothing above it. StatsD is not a wire format here — no histogram semantics worth the name
and no trace story at all.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#the-exporter-is-a-feature-and-core-metrics-is-not" title="The exporter is a feature-gated Native subsystem, and Core\Metrics is Tier 0 in every build"><code>observability/the-exporter-is-a-feature-and-core-metrics-is-not</code></a> <a href="/docs/rules/observability/traces/#four-kinds-become-a-span" title="Exactly four things become a span, and a call event never does"><code>observability/four-kinds-become-a-span</code></a> <a href="/docs/rules/observability/metrics/#a-registry-is-per-core-and-nothing-reads-it" title="A metrics registry is per core, merged at scrape, and no program reads a metric"><code>observability/a-registry-is-per-core-and-nothing-reads-it</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a></dd></div></dl>

</div>

<div class="nv-rule" id="speedscope-timeline-export">

## The four event kinds export as one speedscope evented timeline, beside Callgrind, Clover/lcov and NDJSON

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#speedscope-timeline-export"><code>observability/speedscope-timeline-export</code></a>
</div>

A `TRACE`-flagged run exports its `call`, `gc`, `spawn` and `query` events as speedscope's
evented-profile JSON — open/close pairs at a timestamp, which is exactly what the trace already holds.
speedscope.app opens the file as one scrollable timeline with all four kinds on it, and a spawned
child's events nested under its parent's `spawn` bar.

This is an **additional** export, not a replacement: Callgrind for the aggregate profile, Clover and
lcov for coverage, and Novis-native NDJSON for the raw trace stay as [`testing/debug-surface`](/docs/rules/testing/coverage-and-probes/#debug-surface "Each probe starts and stops mid-request, and exports what existing tooling already reads")
lists them. It reuses the one open format the sampling profiler already commits to rather than
adding a second timeline format with a bespoke viewer to build and maintain.

The NDJSON trace remains the format no third-party tool reads; this export closes that gap for
visualisation, and for nothing else. The exact CLI flag that selects it is left to the exporter's
implementation, the same way the Clover and lcov shapes were.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no Xdebug cachegrind file to open in a bespoke viewer; the timeline is speedscope's open evented format, the same file the sampling profiler writes</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#trace-events-carry-a-kind" title="A trace event carries one of five kinds — call, gc, spawn, query, http — and a call keeps its probe shape"><code>observability/trace-events-carry-a-kind</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-surface" title="Each probe starts and stops mid-request, and exports what existing tooling already reads"><code>testing/debug-surface</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0041.md">record 0041</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a></dd></div></dl>

</div>

<div class="nv-rule" id="memory-is-three-numbers-on-core-budget">

## A request reads its own memory as `memoryHeld`, `memoryPeak` and `memoryLimit` on `Core\Budget`, and the process's resident bytes are `Core\Os`'s

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#memory-is-three-numbers-on-core-budget"><code>observability/memory-is-three-numbers-on-core-budget</code></a>
</div>

A request reads its own memory as three members on `Core\Budget` — `memoryHeld()` is the bytes it
holds right now, `memoryPeak()` the high-water mark of that figure for this request, and
`memoryLimit()` the ceiling both are measured against.

```php
Core\Budget::memoryHeld():  int
Core\Budget::memoryPeak():  int
Core\Budget::memoryLimit(): int
```

**`held`, not `usage`**, because the runtime already says *held*: a breach renders as *"the request
exceeded its memory limit — N bytes held against a ceiling of M"*, and a member whose name disagrees
with the error text about the same quantity is a second vocabulary to learn. `usage` is also the word
that carries PHP's ambiguity between "occupied now" and "consumed in total", and only one of those is
ever meant.

**`memoryLimit` is among them because a peak with no scale is not actionable.** The ceiling is
otherwise reachable only as `Core\Config::get('limits.memory')` — a string with a suffix that every
call site would parse — so the question the trio exists to answer stays one expression:

```php
if (Core\Budget::memoryPeak() * 10 > Core\Budget::memoryLimit() * 9) { … }
```

An uncapped request — `[limits.hard] memory = false`, which
[`config/three-changeability-classes`](/docs/rules/config/changeability-classes/#three-changeability-classes "nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes") permits — answers `0` from `memoryLimit`, the same reading
of zero as "no ceiling" the runtime's own limit check already uses, rather than a second spelling for
it.

**The process's memory is `Core\Os`'s and is a different question.** `Core\Os::residentBytes()` is
the resident set — what `getrusage`'s `ru_maxrss` answers — beside `pid`, `hostname`, `cpuCount` and
`loadAverage`, which are host and process facts too. Two classes, two names, and neither readable as
the other: a per-request figure sitting among host facts would be read as process memory by everyone
who had not been told otherwise, which is PHP's own confusion relocated rather than removed.

There is no `$real_usage`-style boolean in any spelling. Two accountings behind one member is what
[`core-api/no-mode-strings`](/docs/rules/core-api/one-way-to-do-each-thing/#no-mode-strings "A mode is an enum, never a string or an integer constant, and only four grammars are exempt") refuses, and where two numbers are genuinely different questions they
are two members on the two classes that own them.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>$real_usage</code> boolean and no second accounting behind it: <code>memory_get_usage</code>'s two numbers were one member with a flag, where the request's held bytes and the process's resident set are two members on the two classes that own them</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#a-memory-peak-is-recorded-not-asked-for" title="The runtime records a request's memory high-water mark in every build, and a nested isolate restores the enclosing mark rather than clobbering it"><code>observability/a-memory-peak-is-recorded-not-asked-for</code></a> <a href="/docs/rules/observability/traces/#memory-high-water-writes-a-warn" title="A [limits] memory_high_water fraction writes one Warn when a request's memory peak crosses that share of its ceiling, and is off until a block writes one"><code>observability/memory-high-water-writes-a-warn</code></a> <a href="/docs/rules/core-api/one-way-to-do-each-thing/#no-mode-strings" title="A mode is an enum, never a string or an integer constant, and only four grammars are exempt"><code>core-api/no-mode-strings</code></a> <a href="/docs/rules/core-api/what-belongs-in-core/#tier-roster" title="Every subsystem's tier is recorded once in the roster, including the ones no milestone has built"><code>core-api/tier-roster</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#on-limit" title="Tier 1 — a resource limit reaches the request that spent it"><code>errors/on-limit</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0148.md">record 0148</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/migration-members-outstanding.txt"><code>crates/nvs-stdlib/tests/migration-members-outstanding.txt</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/budget.rs"><code>crates/nvs-stdlib/src/budget.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/budget-held-never-exceeds-the-peak-at-any-point-in-a-request.nvst"><code>tests/conformance/core/budget-held-never-exceeds-the-peak-at-any-point-in-a-request.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/budget-the-limit-is-a-ceiling-and-not-a-third-reading.nvst"><code>tests/conformance/core/budget-the-limit-is-a-ceiling-and-not-a-third-reading.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-memory-peak-is-recorded-not-asked-for">

## The runtime records a request's memory high-water mark in every build, and a nested isolate restores the enclosing mark rather than clobbering it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-memory-peak-is-recorded-not-asked-for"><code>observability/a-memory-peak-is-recorded-not-asked-for</code></a>
</div>

The runtime keeps a request's memory high-water mark in every build, updated where the live-byte
balance is updated, and a nested `Ctx` restores the larger of its own mark and the one it displaced
rather than clobbering it.

The peak is **recorded, not sampled and not asked for**. It moves in the same allocator function that
already maintains the live balance, inside the branch that already tests for a positive delta, so it
is exact for every allocation rather than approximate between two reads. A sampled peak would miss
precisely the short spike that deterministic release makes invisible, which is the case the mark
exists for.

**Why a current figure is not enough here, when it nearly is in PHP.** Novis releases memory when the
last reference dies ([`security/arena-is-an-ownership-root`](/docs/rules/security/isolates/#arena-is-an-ownership-root "An isolate's arena is an ownership root, not an address range")), so held bytes fall back toward the
baseline as soon as values die. PHP's allocator keeps its chunks, so a reading taken at the end of a
script is sticky and approximates the high-water mark by accident. A Novis request that decoded a
90 MB payload and returned a 2 KB summary reports the 2 KB, and a request that sat at 96% of its
ceiling for most of its life is indistinguishable at exit from one that never passed 30%. The better
memory behaviour is what destroys the evidence, so the evidence is kept deliberately.

**Nesting saves and restores.** A `Ctx` created inside another rebases the mark to the current
balance and holds the enclosing value; on drop it publishes `max(enclosing, reached)`. Without that,
an isolate that allocated little would erase the peak of the request that spawned it. What the mark
is a peak *of* — one request, or an isolate tree — is
[`security/isolate-budget-is-the-trees`](/docs/rules/security/isolates/#isolate-budget-is-the-trees "A request and everything it spawns share one budget, accounted at the tree's root")' to settle, and this rule inherits that boundary rather
than deciding it.

**It is not resettable.** The peak is evidence an operator needs, and a member that set it back to
the current figure would let an application hide the number the request-level notices exist to
surface. Bounding one section of a program is what `Core\Debug`'s probes are for.

What this spends, per [`programs/memory-priority`](/docs/rules/programs/claims-and-priorities/#memory-priority "Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"): one `isize` per thread and one per `Ctx`.
Nothing per process, nothing that grows with requests served, and nothing that outlives a request.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>memory_get_peak_usage</code> reads a mark PHP keeps because its allocator holds its chunks; here the mark is kept deliberately, because deterministic release means the current figure falls back to the baseline and hides the spike</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#memory-is-three-numbers-on-core-budget" title="A request reads its own memory as memoryHeld, memoryPeak and memoryLimit on Core\Budget, and the process's resident bytes are Core\Os's"><code>observability/memory-is-three-numbers-on-core-budget</code></a> <a href="/docs/rules/observability/traces/#memory-high-water-writes-a-warn" title="A [limits] memory_high_water fraction writes one Warn when a request's memory peak crosses that share of its ceiling, and is off until a block writes one"><code>observability/memory-high-water-writes-a-warn</code></a> <a href="/docs/rules/security/isolates/#arena-is-an-ownership-root" title="An isolate's arena is an ownership root, not an address range"><code>security/arena-is-an-ownership-root</code></a> <a href="/docs/rules/security/isolates/#isolate-budget-is-the-trees" title="A request and everything it spawns share one budget, accounted at the tree's root"><code>security/isolate-budget-is-the-trees</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0148.md">record 0148</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/src/budget.rs"><code>crates/nvs-runtime/src/budget.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/budget-the-peak-outlives-a-spike-the-held-figure-gives-back.nvst"><code>tests/conformance/core/budget-the-peak-outlives-a-spike-the-held-figure-gives-back.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="memory-high-water-writes-a-warn">

## A `[limits] memory_high_water` fraction writes one `Warn` when a request's memory peak crosses that share of its ceiling, and is off until a block writes one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#memory-high-water-writes-a-warn"><code>observability/memory-high-water-writes-a-warn</code></a>
</div>

A `[limits]` block may write `memory_high_water`, a fraction between `0.0` and `1.0`; a request whose
memory peak crosses that share of its effective ceiling writes one `Warn` to `Core\Log` at the end of
the request, naming the peak, the ceiling and the route.

**Unwritten is off, and off is silent** — a deployment gets no high-water log it did not ask for. A
written `0` is a threshold every request passes, not a second spelling of off. A request with no
ceiling at all (`[limits.hard] memory = false`) has no threshold either, because there is nothing for
a fraction to be a fraction of. A value outside `0.0..=1.0` is refused at boot, where it is written,
by the same typed-value path that refuses a malformed size.

**A fraction rather than a size**, so it keeps meaning the same thing under an `[app.limits]` block
that narrows the ceiling ([`config/an-app-block-may-widen-bounded-by-the-global-ceiling`](/docs/rules/config/application-blocks/#an-app-block-may-widen-bounded-by-the-global-ceiling "An [[app]] block may widen a limit or grant a capability up to the global ceiling, and may only lower its own ceiling")). A size
would have to be re-derived for every application block, and would silently stop being a warning at
all for any block that narrowed past it.

**One record per request, at the end of it**, from the mark
[`observability/a-memory-peak-is-recorded-not-asked-for`](/docs/rules/observability/traces/#a-memory-peak-is-recorded-not-asked-for "The runtime records a request's memory high-water mark in every build, and a nested isolate restores the enclosing mark rather than clobbering it") already holds — so this adds no probe
site to the measured path and [`observability/the-runtime-exports-what-it-already-measures`](/docs/rules/observability/metrics/#the-runtime-exports-what-it-already-measures "Production telemetry is read from the instrumentation that already exists, and adds no probe site to the measured path") holds
for it as written.

This is the reading that closes the gap the other two leave. `Script\ExitReport::memoryPeak` requires
an application to have registered a hook, and `nvs_request_memory_peak_bytes` requires an exporter
and someone watching it; the request that matters is the one nobody instrumented and nobody was
watching. A breach needs no equivalent: [`errors/on-limit`](/docs/rules/errors/the-escalation-ladder/#on-limit "Tier 1 — a resource limit reaches the request that spent it")'s report already names both numbers.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP has no warning short of the fatal: a request that touched 96% of <code>memory_limit</code> and returned successfully is indistinguishable from one that never passed 30%</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/observability/traces/#a-memory-peak-is-recorded-not-asked-for" title="The runtime records a request's memory high-water mark in every build, and a nested isolate restores the enclosing mark rather than clobbering it"><code>observability/a-memory-peak-is-recorded-not-asked-for</code></a> <a href="/docs/rules/observability/traces/#a-slow-query-is-logged-past-a-threshold" title="A [db.&lt;name&gt;] slow_query threshold writes a statement's span facts to Core\Log, and is off until a block writes one"><code>observability/a-slow-query-is-logged-past-a-threshold</code></a> <a href="/docs/rules/observability/metrics/#the-runtime-exports-what-it-already-measures" title="Production telemetry is read from the instrumentation that already exists, and adds no probe site to the measured path"><code>observability/the-runtime-exports-what-it-already-measures</code></a> <a href="/docs/rules/config/application-blocks/#an-app-block-may-widen-bounded-by-the-global-ceiling" title="An [[app]] block may widen a limit or grant a capability up to the global ceiling, and may only lower its own ceiling"><code>config/an-app-block-may-widen-bounded-by-the-global-ceiling</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#on-limit" title="Tier 1 — a resource limit reaches the request that spent it"><code>errors/on-limit</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0148.md">record 0148</a></dd></div></dl>

</div>
