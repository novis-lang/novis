# Handoff

## State

**Goal `http-client` — stage 12's connect now walks the whole approved set.** `NvsTcp::connect_racing`
(`crates/nvs-host/src/net.rs:367`) starts an attempt per address `ATTEMPT_DELAY` apart without waiting
for the previous one, keeps the first socket whose handshake is up and drops the rest, all under one
budget; `ask_connected` (`crates/nvs-host/src/net.rs:954`) is the completion test it asks of each and
`wait_any_writable` (`crates/nvs-host/src/net.rs:996`) the park over the set, arming every member under
the task's one token.

`one` (`crates/nvs-stdlib/src/http/transport.rs:1324`) builds a socket per approved address, connects
with the race, files the connection under the address it actually reached, and looks the pool up once
per approved address — a held connection to any member serves the call, which the pooling rule's
fragment now says. Stages 1–11 are on disk behind it.

**What stage 12 still owes:** one test, `a_slow_lookup_leaves_the_core_free_for_another_task`. The other
four of that `-p nvs-stdlib` check are green, and the stage's `-p nvs-runtime` check is on disk
(`crates/nvs-runtime/src/capability.rs:1309`, `:1351`, `:1389`).

Nothing is blocked.

## Next group

**Stage 12: the lookup leaves the core, and stage 13 opens behind it** — one file set:
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **A slow lookup leaves the core free for another task** —
      `rule:http-server/a-core-is-never-blocked-on-a-syscall`, [ADR 0180](../decisions/0180.md) § 14
      first paragraph. `pin` (`crates/nvs-stdlib/src/http.rs:388`) asks `pin_host_addresses`, which
      calls the per-thread seam `install_resolver` files
      (`crates/nvs-runtime/src/capability.rs:289`); the worker installs `resolve_off_core` into it at
      `crates/nvs-host/src/lib.rs:212`. The case installs a resolver that sleeps inside
      `nvs_host::blocking::run`, spawns two tasks on a `Scheduler`, and asserts the second ran while
      the first was still in the lookup. This is the last of stage 12's `-p nvs-stdlib` check.
- [ ] **An outbound call files one `http` trace event with its timings** — stage 13,
      [ADR 0180](../decisions/0180.md) § 15. `sent` (`crates/nvs-stdlib/src/http/transport.rs:1103`)
      is where a call begins and ends, `record_trace` (`crates/nvs-runtime/src/ctx/trace.rs:119`) the
      filing, and `crates/nvs-db/src/span.rs` the `query` event this one is shaped after. Tests
      `an_outbound_call_files_one_http_trace_event_with_its_timings` and
      `a_retried_call_is_one_http_event_carrying_its_attempt_count`.
- [ ] **The `http` event carries nothing a trace may not hold, and a table-answered call files none** —
      same record section, and the reply keeps no timing member.
      `crates/nvs-runtime/src/ctx/trace.rs:78` is what a `TraceEvent` may hold; the existing case
      `no_trace_span_or_error_message_carries_a_header_value` in
      `crates/nvs-stdlib/src/http/transport.rs` is the shape to extend. Tests
      `the_http_trace_event_carries_no_query_string_header_value_or_body` and
      `a_call_the_table_answered_files_no_http_event`.

## Backlog

- Stage 14 is the rulebook tail: three `tools/rules.py --show … shipped` checks —
  `docs/agent/loop-goal.toml:9684`.
- `Core\Mail` still connects to one resolved address (`crates/nvs-stdlib/src/mail.rs:715`), which this
  goal does not cover — `rule:http-server/an-outbound-call-tries-every-approved-address` is HTTP's.
- `Core\Http\Target` keeps no members, and nothing in stage 12 changes that —
  `rule:http-server/allow-url-pins-the-address`.
- The REST package, OAuth, HTTP/2 and a resolver of Novis's own stay out —
  `docs/agent/loop-goal.md` § *Standing decisions*.
