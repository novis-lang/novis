# Handoff

## State

**Goal `http-client` — stage 12 is closed.** The launderer's lookup leaves the core: `pin`
(`crates/nvs-stdlib/src/http.rs:388`) asks `pin_host_addresses`, which reaches the per-thread seam
`install_resolver` files (`crates/nvs-runtime/src/capability.rs:289`), and a worker puts
`resolve_off_core` — a `nvs_host::blocking::run` around the real lookup — into it at
`crates/nvs-host/src/lib.rs:212`. Both of stage 12's checks are green: the `-p nvs-runtime` three at
`crates/nvs-runtime/src/capability.rs:1309`, `:1351`, `:1389`, and the `-p nvs-stdlib` five, whose last
one is now `crates/nvs-stdlib/src/http.rs:4211`.

Stages 1–11 are on disk behind it. Stage 13 is untouched and nothing is blocked.

## Next group

**Stage 13: the `http` trace event** — one file set: `crates/nvs-runtime/src/ctx/trace.rs`,
`crates/nvs-stdlib/src/http/transport.rs`. The goal's own prose calls this stage its own session, and
the reason is the first item: the resolve timing is measured in `Core\Http::allowUrl`, not in the
transport, so where that number crosses into the event is a decision the stage opens with.

- [ ] **A fifth `TraceKind`, `http`, with the stage's fixed field set** —
      `rule:observability/trace-events-carry-a-kind`, [ADR 0180](../decisions/0180.md) § 13. The enum is
      `crates/nvs-runtime/src/ctx/trace.rs:54`, the `query` kind's filing beside it at
      `crates/nvs-runtime/src/ctx/trace.rs:149`, and `crates/nvs-db/src/span.rs:1` is the precedent for
      how a kind fixes its fields. Fields: method; scheme, host and port; path without its query;
      status; attempt count and redirect hops; the address connected to; and the resolve, connect, TLS,
      first-byte and total times.
- [ ] **An outbound call files one `http` event with its timings** — the same rule. Filed once per call
      from `send` (`crates/nvs-stdlib/src/http/transport.rs:1135`), which is where the attempt loop
      `attempts` (`crates/nvs-stdlib/src/http/transport.rs:1271`) and the per-attempt exchange `one`
      (`crates/nvs-stdlib/src/http/transport.rs:1324`) both report into, so a retried call is one event
      carrying its attempt count. Tests `an_outbound_call_files_one_http_trace_event_with_its_timings`
      and `a_retried_call_is_one_http_event_carrying_its_attempt_count`.
- [ ] **The event carries nothing a trace may not hold, and a table-answered call files none** —
      `rule:security/secret-sinks-refuse` over a trace, and
      `rule:testing/an-outbound-call-is-answered-from-a-table`. No query string, header value or body.
      The table arm is `exchanged` (`crates/nvs-stdlib/src/http.rs:3182`), which hands off to `faked`
      (`crates/nvs-stdlib/src/http.rs:3453`) before `transport::send`
      (`crates/nvs-stdlib/src/http/transport.rs:1135`) is reached at all — so a table-answered call
      crossed no network and files nothing, and the filing belongs below that branch. Tests
      `the_http_trace_event_carries_no_query_string_header_value_or_body` and
      `a_call_the_table_answered_files_no_http_event`.

## Backlog

- Stage 14 flips stage 2's eleven rules to `shipped` and re-renders — `docs/agent/loop-goal.md` § *Stage 14*.
- `Core\Http\Response` gains no timing member; the trace is the one surface — goal § *Standing decisions*.
- Parsing `Link`, `Retry-After` for a program, and RFC 9457 problem details are the `nvs/rest` package's.
