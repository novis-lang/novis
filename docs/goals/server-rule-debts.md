---
looper: 1
title: The server and configuration rules that are designed are built
summary: "Every rule this goal owns is `shipped` with a guard test, and `bun nv owners --closes server-rule-debts` passes."
context: []
stages:
  - id: 1
    title: "`request_body` and `upload_total` are configuration rows"
    status: done
    context: [crates/nvs-server/src/body.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-config/src/mode.rs]
    extra:
      context:
        rules: [http-server/request-body-and-upload-total-are-two-caps, config/three-changeability-classes, config/ceilings-are-their-own-directives, http-server/upload-total-is-enforced-on-the-wire]
        shapes: []
  - id: 2
    title: the access log
    status: done
    context: [crates/nvs-config/src/mode.rs, crates/nvs-server/src/trace.rs]
    extra:
      context:
        rules: [http-server/the-access-log-is-a-mode-default, config/a-mode-is-five-defaults, errors/log-fields, errors/renderings]
        shapes: []
  - id: 3
    title: "a Unix spelling with no `AF_UNIX` transport refuses at boot"
    status: done
    context: [crates/nvs-config/src/resolve.rs, crates/nvs-config/src/store.rs, crates/nvs-db/src/conn.rs]
    extra:
      context:
        rules: [config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot, config/unix-scheme-in-a-url-and-a-bare-path-in-a-host]
        shapes: [A diagnostic]
  - id: 4
    title: the resolved root is announced and stored
    status: done
    context: [crates/nvs-cli/src/config.rs, crates/nvs-cli/src/reload.rs]
    extra:
      context:
        rules: [config/the-resolved-root-is-announced-and-stored, config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults]
        shapes: []
  - id: 5
    title: "an upgrade takes `limits`, `grants` and `on`"
    status: done
    context: [crates/nvs-stdlib/src/socket.rs]
    extra:
      context:
        rules: [concurrency/an-upgrades-options-are-spawn-scripts, concurrency/a-connection-is-a-root-isolate]
        shapes: ["A `.nvst` test case"]
  - id: 6
    title: deferred work cannot write the response
    status: open
    context: [crates/nvs-runtime/src/deferred.rs, crates/nvs-runtime/src/later.rs]
    extra:
      context:
        rules: [concurrency/deferred-work-cannot-write-the-response, concurrency/after-response-outlives-the-connection]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 7
    title: a foreign-connection enqueue is counted
    status: open
    context: [crates/nvs-stdlib/src/queue.rs]
    extra:
      context:
        rules: [concurrency/foreign-connection-enqueue-is-counted, concurrency/enqueue-commits-with-your-write]
        shapes: ["A `.nvst` test case"]
  - id: 8
    title: "`receive()` is the one wait"
    status: open
    context: [crates/nvs-stdlib/src/socket.rs, crates/nvs-stdlib/src/topic.rs]
    extra:
      context:
        rules: [concurrency/receive-is-the-one-wait, core-classes/topic]
        shapes: []
  - id: 9
    title: the trace id is on the response
    status: open
    context: [crates/nvs-server/src/trace.rs]
    extra:
      context:
        rules: [observability/a-trace-id-exists-for-every-request, observability/sampling-is-head-based]
        shapes: []
checks:
  - id: body-caps-are-rows
    stage: 1
    name: "nvs-config (`[limits] request_body` and `upload_total` resolve with their defaults, and a value past its `[limits.hard]` ceiling is refused)"
    run: [cargo, test, --test, directives]
    tests: [request_body_and_upload_total_resolve_to_their_defaults, a_body_cap_past_its_hard_ceiling_is_refused]
  - id: body-caps-reach-the-door
    stage: 1
    name: "a configured `request_body` and `upload_total` bound the body the door reads"
    run: [cargo, test, --test, request]
    tests: [a_configured_request_body_bounds_a_parsed_body, a_configured_upload_total_bounds_a_streamed_body]
  - id: access-log
    stage: 2
    name: "nvs-server (an access line carries its fields and the trace id, the health probe writes none, and a `5xx` is logged with `access` off)"
    run: [cargo, test, --lib, "trace::"]
    tests: [an_access_line_carries_method_path_status_duration_bytes_and_trace_id, the_health_probe_writes_no_access_line, a_5xx_is_logged_with_access_off]
  - id: unix-spelling-at-boot
    stage: 3
    name: "nvs-config (a `[db]` Unix spelling on a build with no `AF_UNIX` transport is refused at boot where it was written)"
    run: [cargo, test, --test, db]
    tests: [a_db_unix_spelling_with_no_unix_transport_is_refused_at_boot]
  - id: resolved-root
    stage: 4
    name: "nvs-cli (boot prints the resolved root and every file the tree reached, and a reload reads the stored absolute path)"
    run: [cargo, test, --test, live_config]
    tests: [boot_prints_the_resolved_root_and_every_file_it_reached, a_reload_reads_the_stored_path_after_the_working_directory_changes, the_shipped_defaults_line_is_printed_when_no_file_is_found]
  - id: upgrade-options
    stage: 5
    name: "an upgrade narrows its grants, takes its own limits and starts on its core"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/socket-an-upgrade-narrows-its-grants.nvst, --case, tests/conformance/core/socket-an-upgrade-takes-its-own-limits.nvst]
    tests: [tests/conformance/core/socket-an-upgrade-narrows-its-grants.nvst, tests/conformance/core/socket-an-upgrade-takes-its-own-limits.nvst]
  - id: deferred-response-write
    stage: 6
    name: "a `Core\\Response` write in deferred work is a compile error where it is visible and a throw otherwise"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/after-response-a-response-write-does-not-compile.nvst, --case, tests/conformance/core/after-response-a-hidden-response-write-throws.nvst]
    tests: [tests/conformance/core/after-response-a-response-write-does-not-compile.nvst, tests/conformance/core/after-response-a-hidden-response-write-throws.nvst]
  - id: foreign-enqueue-counted
    stage: 7
    name: "`stats` counts a push made while another connection's transaction is open"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/queue-stats-counts-a-push-outside-the-callers-transaction.nvst]
    tests: [tests/conformance/core/queue-stats-counts-a-push-outside-the-callers-transaction.nvst]
  - id: receive-one-wait
    stage: 8
    name: "nvs-stdlib (a `receive` already parked on the peer wakes for a topic delivery)"
    run: [cargo, test, --lib, "socket::"]
    tests: [a_receive_parked_on_the_peer_wakes_for_a_delivery, a_delivery_from_another_core_wakes_a_parked_receive]
  - id: trace-id-on-the-response
    stage: 9
    name: "nvs-server (every response carries the request's trace id in one header, sampled or not, and an inbound `X-Request-ID` appears nowhere)"
    run: [cargo, test, --lib, "trace::"]
    tests: [every_response_carries_its_trace_id_sampled_or_not, an_inbound_x_request_id_appears_nowhere]
  - id: owners-closes
    stage: 9
    name: no designed rule or gap names this goal
    run: [bun, nv, owners, --closes, server-rule-debts]
    reach: true
---

# The server and configuration rules that are designed are built

## Why

Every rule in the rulebook is `shipped` or `designed`, and every `designed` one names the goal or the
milestone that builds it. These nine rules describe the server and its configuration, and each is
part built: two body caps are constants, a mode row nobody reads, a refusal that waits for the first
connection, a root that is never announced, three upgrade options that are refused by name, deferred
work that can still reach the response, a counter that does not exist, a wait that a delivery cannot
wake, and a trace id the response does not carry. This goal builds each and flips its rule to
`shipped`.

It is a chain goal after `request-isolation-debts`, because isolation outranks the rest.

## Standing decisions

**A stage ships its rule.** The stage that builds a rule sets its record's `status` to `shipped`,
removes its `goal` owner, fills `guardedBy` with the tests the stage's checks name
(`data/rules/<topic>/<slug>.json`), and rewrites the fragment whole so it says what is built. Home:
the rule's own fragment.

**The body caps** become `[limits] request_body` and `[limits] upload_total`, `Runtime` rows with
`[limits.hard]` ceilings, at the defaults and ceilings the rule's table gives. The two constants go:
`crates/nvs-server/src/body.rs:60` and `crates/nvs-stdlib/src/request.rs:2832`. `nvs config dump`
prints both where the tree writes them; no command prints a limit's default.

**The access log** is written by the server for every request when `[log] access` is true, through the
one diagnostic record, with the fields the rule names and the trace id. The health probe writes none.
A `5xx` is logged whatever `access` says. No Common or Combined Log Format. The `log.access` row
already exists (`crates/nvs-config/src/mode.rs:85-89`) and nothing reads it.

**A Unix spelling** is refused at boot for every key that can carry one. The `[cache.shared] url` is
refused at boot already (`crates/nvs-config/src/store.rs:93`); a `[db]` connection's `unix:` url or bare
path is refused only at connect time (`crates/nvs-db/src/conn.rs:135`). The `[db]` refusal moves to
`resolve`, beside the store's (`crates/nvs-config/src/resolve.rs:532`), with the same code `E0635`.
The connect-time refusal stays as the last line of defence.

**The resolved root** is canonicalized once at boot and stored. A reload reads the stored path
(`crates/nvs-cli/src/reload.rs:137-139` keeps `--config` as written today). Boot prints the lines the
rule's example shows, and the declined reason of a data-folder write joins them
(`crates/nvs-cli/src/config.rs:536-539`).

**An upgrade's options** are `args`, `limits`, `grants` and `on`, with `spawn script`'s meaning. The
refusal by name (`crates/nvs-stdlib/src/socket.rs:174-186`) is replaced by the parameters, never by
accepting and dropping them. `grants` narrows against the upgrading request's own overlay. The
session's call on `on`: a connection's socket is core-local, so `on: "here"` is accepted and
`on: "worker"` does not compile (`E0818`). The rule's fragment is the home.

**Deferred work** gets a compile error for a `Core\Response` write inside an `afterResponse` callable
where the checker sees it, and a throw at run time otherwise. `onUncaughtThrow` already does not fire
for deferred work (`crates/nvs-runtime/src/deferred.rs:216`). `make_slotted`'s refusal
(`crates/nvs-runtime/src/later.rs:222-228`) is the shape the throw follows.

**A foreign-connection enqueue** is counted where `announce` already tells the case apart
(`crates/nvs-stdlib/src/queue.rs:3590-3608`). The writer's call: the count is a column on the job row,
so it survives a restart and counts across processes, and `stats` reports it as `nonTransactional`.
Memory: one small column per job row; no process memory.

**`receive()`** parks on the peer read and on the connection's delivery queue at once, and wakes on
whichever arrives first. Today a delivery cannot wake a park that is already running
(`crates/nvs-stdlib/src/socket.rs:1017-1019`, `nvs_runtime::Ctx::deliver`'s gap). The overflow check
stays before the wait.

**The trace id on the response.** `rule:observability/a-trace-id-exists-for-every-request` claims the
header and owns this stage.
The writer's call on the name: W3C Trace Context's `traceresponse` header, so a proxy and a tracing
backend read one standard header. It is written on every response, sampled or not. An inbound
`X-Request-ID` stays ignored.

**Feature proofs.** A stage that changes a `Core` member updates that member's feature proofs
(`rule:testing/feature-proofs`; `bun nv proofs --id '<member>'` prints what is owed): Stage 5
`Core\Socket::upgrade`, Stage 7 `Core\Queue::stats`, Stage 8 `Core\Socket::receive`, Stage 9
`Core\Server::traceId`.

## Stages

### 1. `request_body` and `upload_total` are configuration rows

**Does:** the two body caps are `[limits]` rows with `[limits.hard]` ceilings, and the constants go.

- [x] The two rows and their ceilings in `nvs-config`. `crates/nvs-config/src/mode.rs:85`
- [x] The door reads `upload_total` from the request's limits. `crates/nvs-server/src/body.rs:54`
  - [x] The door takes the global `[limits] upload_total` at both call sites.
  - [x] A reader's claim sets the request's own `upload_total` on the wire (`RequestBody::allow_total`).
- [x] The readers read `request_body` from the request's limits. `crates/nvs-stdlib/src/request.rs:2832`
  - [x] `body()`, the form readers, `readAll()` and multipart fields read `Ctx::request_body_limit`.
  - [x] The two `cargo test --test request` cases pass, and the rule is shipped.

### 2. the access log

**Does:** `[log] access` writes one access line per request, and a `5xx` is always logged.

- [x] The server reads `log.access` and writes the line through the diagnostic record. `crates/nvs-config/src/mode.rs:86`
  - [x] `[log] access` is a key: the tree field, the `ByMode` default row and both `default.toml` blocks.
  - [x] `trace::access` writes the record, and the door calls it where it counts the request.
- [x] The trace id and the route name on the line, and the health probe skipped. `crates/nvs-server/src/trace.rs:537`
  - [x] `Reply::health` marks its response with `Probe`, and a marked response writes no line.
  - [x] The three `trace::` cases pass, and the rule is shipped.

### 3. a Unix spelling with no `AF_UNIX` transport refuses at boot

**Does:** a `[db]` Unix spelling on a build with no Unix transport is refused at boot.

- [x] The `[db]` check beside the store's. `crates/nvs-config/src/resolve.rs:532`
- [x] The connect-time refusal's comment says it is the last line. `crates/nvs-db/src/conn.rs:135`

### 4. the resolved root is announced and stored

**Does:** boot prints the resolved root and every file it reached, and a reload reads the stored path.

- [x] Canonicalize and store the root at boot, and print the lines. `crates/nvs-cli/src/config.rs:537`
- [x] A reload reads the stored path. `crates/nvs-cli/src/reload.rs:137`
- [x] The declined reason joins the boot lines. `crates/nvs-cli/src/config.rs:1334`

### 5. an upgrade takes `limits`, `grants` and `on`

**Does:** `Core\Socket::upgrade` takes the three narrowing options with `spawn script`'s meaning.

- [x] The three parameters, forwarded to the connection isolate. `crates/nvs-stdlib/src/socket.rs:174`
- [x] The same options on `Core\Sse::upgrade` if it opens a connection isolate too. `crates/nvs-stdlib/src/sse.rs:145`
- [x] The conformance cases and the feature proofs of `Core\Socket::upgrade`. `docs/examples/core/Socket/upgrade/about.md:1`

### 6. deferred work cannot write the response

**Does:** a `Core\Response` write in deferred work does not compile where it is visible, and throws otherwise.

- [ ] The run-time throw. `crates/nvs-runtime/src/later.rs:222`
- [ ] The compile error, with its diagnostic. `crates/nvs-runtime/src/deferred.rs:216`

### 7. a foreign-connection enqueue is counted

**Does:** `Core\Queue::stats` reports how many pushes committed outside the caller's transaction.

- [ ] The job row records the case, and `stats` reads it. `crates/nvs-stdlib/src/queue.rs:3604`
- [ ] The conformance case and the feature proofs of `Core\Queue::stats`. `docs/examples/core/Queue/stats/about.md:1`

### 8. `receive()` is the one wait

**Does:** a parked `receive()` wakes for a peer frame or a topic delivery, whichever comes first.

- [ ] The park waits on both sources. `crates/nvs-stdlib/src/socket.rs:1035`
- [ ] `Ctx::deliver` wakes a running park, and its gap goes. `crates/nvs-stdlib/src/socket.rs:1017`

### 9. the trace id is on the response

**Does:** every response carries the request's trace id in the `traceresponse` header.

- [ ] The header on every response, sampled or not. `crates/nvs-server/src/trace.rs:398`
- [ ] The feature proofs of `Core\Server::traceId`. `docs/examples/core/Server/traceId/about.md:1`

## Handoff

### State

Stages 1 to 5 are done. `Core\Socket::upgrade` and `Core\Sse::upgrade` take `limits:`, `grants:`
and `on:`. `crate::socket::upgrade_narrowing` keeps the written grants the request holds, and
`nvs_runtime::Upgrade` carries the `Narrowing` to every door that starts a connection, through
`Isolate::narrowed_by`. `on: "worker"` does not compile (`E0818`), because a connection's socket
is core-local. `rule:concurrency/an-upgrades-options-are-spawn-scripts` is `shipped`, and the
perf records of both classes are measured again.

A grant the request gave up is not observable in a `.nvst` case, because a request cannot narrow
its own grants. `an_upgrades_grants_keep_only_what_the_request_holds` in
`crates/nvs-stdlib/src/socket.rs` covers that half.

The nightly reds ran on `origin/main`, far behind local `main`, and nothing here pushes.

### Next

**Stage 6, deferred work cannot write the response**: `crates/nvs-runtime/src/later.rs`,
`crates/nvs-runtime/src/deferred.rs`, `tests/conformance/core/`
(`rule:concurrency/deferred-work-cannot-write-the-response`).

- [ ] The run-time throw for a `Core\Response` write in after-response work, in `make_slotted`'s shape. `crates/nvs-runtime/src/later.rs:222`
- [ ] The compile error where the checker sees the write inside an `afterResponse` callable, with its diagnostic. `crates/nvs-runtime/src/deferred.rs:216`
- [ ] The two `.nvst` cases the `deferred-response-write` check names. `crates/nvs-runtime/src/deferred.rs:205`

### Backlog

- Stages 7 to 9: this file's § *Stages*.
- `on: "worker"` for a connection, if the rule should promise it: `crates/nvs-types/src/expr/isolate.rs`.
- A reload keeps the boot's canonical roots, so a symlink swapped to a new release is not followed until restart, if the rule should promise otherwise: `crates/nvs-cli/src/config.rs`.
- A program's `Core\Config::set` of `[log] access` does not reach the access line, if the rule should promise it: `crates/nvs-server/src/trace.rs`.
- The nightly reds, the macOS warm-start budget among them, checked against local `main` once `origin/main` is pushed: `.github/workflows/`, `tools/nv/cmd/bench.ts`.
- Every socket leg's OS error 95 on Windows: `crates/nvs-db/src/matrix.rs`.
