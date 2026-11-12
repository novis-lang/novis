---
looper: 1
title: "every wait ends at its deadline, set in one place, and every upgraded connection is watched"
summary: "Reached when the park funnel ends every request's wait at its wall deadline with the limit's FATAL and no call site clamps by hand, a wait that outlives wall_time names why, a WebSocket or event-stream handler is charged cpu_time for each turn, an event stream takes a max_open place, and a peer that only keeps bytes arriving is ended."
context: [crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/timer.rs, crates/nvs-runtime/src/ctx/limits.rs, docs/rules/errors/on-limit.md]
stages:
  - id: 1
    title: "every wait that is not a request's own names why it outlives wall_time"
    status: done
    context: [crates/nvs-host/src/block_on.rs, crates/nvs-host/src/blocking.rs, crates/nvs-host/src/group.rs, docs/rules/concurrency/the-parking-contract.md]
  - id: 2
    title: the park funnel bounds every wait
    status: open
    context: [crates/nvs-runtime/src/budget.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/abi.rs, crates/nvs-host/src/net.rs]
  - id: 3
    title: "the hand clamps are deleted, crate by crate"
    status: open
    context: [crates/nvs-stdlib/src/db/execute.rs, crates/nvs-stdlib/src/http.rs, crates/nvs-runtime/src/peer.rs, docs/rules/core-classes/db-statement-members.md]
  - id: 4
    title: "a stream that never runs dry meets its bound, and the pool skips an abandoned job"
    status: open
    context: [crates/nvs-host/src/net.rs, crates/nvs-host/src/reactor.rs, crates/nvs-host/src/blocking.rs, crates/nvs-host/src/slice.rs]
  - id: 5
    title: "an upgraded connection is watched, takes a place and is ended by its idle timeout"
    status: open
    context: [crates/nvs-host/src/isolate.rs, crates/nvs-host/src/watchdog.rs, crates/nvs-server/src/socket.rs, crates/nvs-server/src/bounds.rs, docs/rules/concurrency/connection-bounds-are-finite.md, docs/rules/concurrency/a-connection-is-a-root-isolate.md]
  - id: 6
    title: a wait at request end is bounded
    status: open
    context: [crates/nvs-cli/src/extensions.rs, crates/nvs-host/src/block_on.rs, crates/nvs-ext/src/call.rs, docs/rules/packaging/a-fresh-instance-per-request.md]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: waits-past-wall-time-are-named
    stage: 1
    name: "every park that outlives wall_time names a reason, and the test reads the sources to find one that does not"
    run: [cargo, test, --test, park_sites, --, every_wait_past_wall_time_names_a_reason_at_a_listed_site, only_the_funnel_suspends_a_task]
    timeout: 1800
    tests:
      - every_wait_past_wall_time_names_a_reason_at_a_listed_site
      - only_the_funnel_suspends_a_task
  - id: the-funnel-bounds-a-park
    stage: 2
    name: "a park ends at its tree's wall deadline with the limit's FATAL, keeps one timer entry, and a named wait stays parked"
    run: [cargo, test, --lib, --, a_park_with_no_deadline_ends_at_its_trees_wall_deadline, a_park_with_a_later_deadline_ends_at_the_wall_deadline_on_one_timer_entry, a_named_wait_stays_parked_past_the_wall_deadline, a_limit_handler_that_parks_is_bounded_by_its_slice_end, a_wait_that_ended_at_the_wall_deadline_runs_the_handler_then_is_fatal, a_member_that_fails_past_the_wall_deadline_reports_the_wall_time_fatal, a_park_under_no_wall_time_arms_no_timer_entry]
    timeout: 1800
    tests:
      - a_park_with_no_deadline_ends_at_its_trees_wall_deadline
      - a_park_with_a_later_deadline_ends_at_the_wall_deadline_on_one_timer_entry
      - a_named_wait_stays_parked_past_the_wall_deadline
      - a_limit_handler_that_parks_is_bounded_by_its_slice_end
      - a_wait_that_ended_at_the_wall_deadline_runs_the_handler_then_is_fatal
      - a_member_that_fails_past_the_wall_deadline_reports_the_wall_time_fatal
      - a_park_under_no_wall_time_arms_no_timer_entry
  - id: no-hand-clamp-is-left
    stage: 3
    name: "no member clamps its wait to the wall deadline by hand, and a statement's wait ends with the FATAL and no catchable error first"
    run: [cargo, test, --test, park_sites, --, no_member_clamps_its_wait_to_the_wall_deadline_by_hand]
    timeout: 1800
    tests:
      - no_member_clamps_its_wait_to_the_wall_deadline_by_hand
  - id: a-statement-ends-with-the-fatal
    stage: 3
    name: "a statement whose reply never comes ends at the wall deadline with the wall-time FATAL"
    run: [cargo, test, --lib, --, a_statement_wait_that_ends_at_the_wall_deadline_is_the_wall_time_fatal]
    timeout: 1800
    tests:
      - a_statement_wait_that_ends_at_the_wall_deadline_is_the_wall_time_fatal
  - id: a-busy-stream-is-bounded
    stage: 4
    name: "a stream whose reads never block ends at its deadline or its tree's stop and shares its core, a hang-up is read at once, and an abandoned job is skipped"
    run: [cargo, test, --lib, --, a_stream_that_never_runs_dry_ends_at_its_deadline, a_stream_that_never_runs_dry_ends_when_its_tree_is_stopped, a_stream_that_never_runs_dry_gives_the_core_to_a_waiting_task, a_read_that_parks_pays_no_streak_check, a_reply_of_endless_notices_ends_at_the_statement_deadline, a_short_read_that_came_with_a_hang_up_reads_once_more, a_job_abandoned_in_the_queue_is_skipped_when_it_is_dequeued]
    timeout: 1800
    tests:
      - a_stream_that_never_runs_dry_ends_at_its_deadline
      - a_stream_that_never_runs_dry_ends_when_its_tree_is_stopped
      - a_stream_that_never_runs_dry_gives_the_core_to_a_waiting_task
      - a_read_that_parks_pays_no_streak_check
      - a_reply_of_endless_notices_ends_at_the_statement_deadline
      - a_short_read_that_came_with_a_hang_up_reads_once_more
      - a_job_abandoned_in_the_queue_is_skipped_when_it_is_dequeued
  - id: an-upgraded-connection-is-bounded
    stage: 5
    name: "a connection handler that never waits ends at cpu_time on both doors, the count restarts each turn, an event stream takes a max_open place, and pings do not restart idle_timeout"
    run: [cargo, test, --lib, --, a_websocket_handler_that_never_waits_ends_at_its_cpu_limit, an_event_stream_handler_that_never_waits_ends_at_its_cpu_limit, a_connections_cpu_count_starts_again_at_each_turn, an_event_stream_takes_a_max_open_place_and_gives_it_back, an_event_stream_past_max_open_is_answered_503, a_peer_that_sends_only_pings_is_closed_at_the_idle_timeout, a_peer_that_sends_only_empty_fragments_is_closed_at_the_idle_timeout]
    timeout: 1800
    tests:
      - a_websocket_handler_that_never_waits_ends_at_its_cpu_limit
      - an_event_stream_handler_that_never_waits_ends_at_its_cpu_limit
      - a_connections_cpu_count_starts_again_at_each_turn
      - an_event_stream_takes_a_max_open_place_and_gives_it_back
      - an_event_stream_past_max_open_is_answered_503
      - a_peer_that_sends_only_pings_is_closed_at_the_idle_timeout
      - a_peer_that_sends_only_empty_fragments_is_closed_at_the_idle_timeout
  - id: request-end-is-bounded
    stage: 6
    name: "a resource destructor that sends or waits at request end is ended at its bound, and a guest call parked past wall_time ends with the FATAL"
    run: [cargo, test, --lib, --, a_destructor_that_sends_at_request_end_is_refused_and_the_request_ends, a_destructor_that_waits_at_request_end_is_dropped_at_its_bound, a_guest_call_parked_past_the_wall_deadline_ends_with_the_wall_time_fatal]
    timeout: 1800
    tests:
      - a_destructor_that_sends_at_request_end_is_refused_and_the_request_ends
      - a_destructor_that_waits_at_request_end_is_dropped_at_its_bound
      - a_guest_call_parked_past_the_wall_deadline_ends_with_the_wall_time_fatal
---

# every wait ends at its deadline, set in one place, and every upgraded connection is watched

## Why

A parked task reaches no safepoint poll, and the watchdog stops only a request a core is running
(`crates/nvs-host/src/watchdog.rs:173`). So each waiting member ends its own wait at the request's
`[limits] wall_time` by hand. `Ctx` has five methods for it (`crates/nvs-runtime/src/ctx/limits.rs:1048`
to `:1089`), called at about sixty sites in more than twenty files. Each site clamps before its wait
and checks after it. The endings differ: most give the wall-time `FATAL`, and a statement on a wire
driver gives a catchable error first (`crates/nvs-stdlib/src/db/execute.rs:92`). A new member that
waits compiles and passes with no clamp. Its wait then outlives `wall_time`, and a `host.park(None)`
waits for as long as its peer stays silent. Every park already passes one function, `yield_on`
(`crates/nvs-host/src/scheduler.rs:1857`), and that function reads no deadline.

An upgraded connection has less. Both doors start the connection's isolate with no `watched_by`
(`crates/nvs-server/src/serve.rs:2553`, `crates/nvs-server/src/serve.rs:2824`), so its tree is never
published on its core. `Meter::switch` charges only a published tree
(`crates/nvs-host/src/watchdog.rs:437`), and the sweep walks publications only. b1ce9fe64f took
`wall_time` off the connection and wrote into two rule fragments that `cpu_time` still applies. Nothing
applies it. A WebSocket or event-stream handler that loops on a client's message holds its core for
good, and one message per core stops the server. Any remote client of a route that upgrades reaches it.

Three more holes are beside it, all reachable by a peer:

- A WebSocket peer that keeps pings or empty fragments arriving is never ended. `Framed::receive_or`
  arms `idle_timeout` again after every control frame (`crates/nvs-server/src/socket.rs:409`,
  `crates/nvs-server/src/socket.rs:417`). `NvsStream::read` reads its deadline only after `WouldBlock`
  (`crates/nvs-host/src/net.rs:1801`), so the task never parks and its core runs nothing else.
- An event stream takes no `max_open` place. `Slot::take` has one caller, the WebSocket codec
  (`crates/nvs-server/src/socket.rs:246`), and the event-stream door takes none
  (`crates/nvs-server/src/serve.rs:2543`). Open streams have no cap but file descriptors.
- A hostile database that answers with an endless run of small notices keeps the same read loop busy
  (`crates/nvs-db/src/pg.rs:3160`). No Novis code runs there, so no poll sees `wall_time` or `cpu_time`.

Three waits end late or never, and the program's own code or a slow host reaches them:

- An extension's resource destructor runs at request end with nobody to send its HTTP request
  (`crates/nvs-cli/src/extensions.rs:486`). The drive parks and ignores a cancellation
  (`crates/nvs-host/src/block_on.rs:164`), so the task, its tree and its instance place leak.
- 806f0f2fc2 lets a task abandon a blocking-pool job at its deadline. The job reads that only after
  it has run (`crates/nvs-host/src/blocking.rs:520`), so a slow resolver's queue outlives its requests.
- 649760357a parks the next read after a short read with no `recv` (`crates/nvs-host/src/net.rs:1573`).
  A hang-up that arrived in the same event is then read only at the phase deadline.

## Standing decisions

**`yield_on` bounds every `Waiting::Parked` by the tree's wall deadline.** The bound is the limit
handler's slice end while one is open, and the tree's wall deadline otherwise
(`crates/nvs-runtime/src/ctx/safepoint.rs:708`, `:719`). The funnel reads it from the tree whose turn
the stack is, which `ArmedStack::take` already holds there (`crates/nvs-runtime/src/budget.rs:658`).
It arms the task's one timer entry at the earlier of the bound and what the site armed, and puts the
site's entry back on resume. A bound already passed parks nothing. **What it spends:** per park, one
load for a request with no `wall_time`. Under a `wall_time` it is one load and one clock read per
park, which the hand clamp pays today. A park that passed no deadline adds one timer insert and one
removal. No memory. Safe default if the turn's tree is not reachable for a child placed on another
core: read it from the view that child published (`crates/nvs-host/src/watchdog.rs:668`). Declined: a
watchdog sweep over parked tasks, which costs every sweep and stops a wait one interval late.

**A wait that hits the bound has one ending, the one `rule:errors/on-limit` states.** The funnel
claims the tree's wall stop as the watchdog does (`crates/nvs-runtime/src/ctx/safepoint.rs:746`),
raises `WALL_TIME` and answers `Resumed::Cancelled`. Every member already turns that into
`Ctx::cancel`, which runs the poll (`crates/nvs-runtime/src/ctx/safepoint.rs:560`). The poll runs the
`Core\Fatal::onLimit` handler in its slice and ends the request with the wall-time `FATAL`
(`crates/nvs-runtime/src/ctx/safepoint.rs:1226`). No member's own catchable timeout comes first, the
statement wait included. `Ctx::cancel` does not record a cancellation when the stop is the wall
time's, so a group still tells a stopped child from a failed one. A wait with no `Ctx`, such as a
socket read inside a driver, returns its I/O error. `run_helper`'s error arm then reports the wall-time
breach in place of the member's error, as it does for memory (`crates/nvs-runtime/src/abi.rs:615`).
**What it spends:** one load and, under a `wall_time`, one clock read per member call that fails.
Teardown I/O after the stop fails at once, so a pooled connection is closed and not returned, as now.
Declined: a new `Resumed` answer for the deadline, because every park site would have to handle it.

**A wait that outlives `wall_time` says so with one argument.** `Waiting` gains
`ParkedPast(PastWallTime)`, and the enum's variants are the whole list of reasons. `Parked` stays the
default, so a new wait is bounded unless its author names a reason. The reasons and their sites:

- `ServerTask`, a task that serves no request: the accept loop's tail
  (`crates/nvs-server/src/serve.rs:3579`), the ticker (`crates/nvs-server/src/schedule.rs:730`,
  `:801`), the spool sealer (`crates/nvs-cli/src/serve.rs:1708`), the receptionist
  (`crates/nvs-host/src/worker.rs:1273`) and a queue worker's idle wait.
- `ConnectionDrive`, the connection's own task: `block_on`'s two parks
  (`crates/nvs-host/src/block_on.rs:235`, `:270`) and the wait for the program's answer
  (`crates/nvs-server/src/serve.rs:1376`). The connection's phase deadlines bound the first, and the
  program's own stop ends the second.
- `Join`, a wait for the task's own children: `crates/nvs-host/src/isolate.rs:1002`,
  `crates/nvs-host/src/group.rs:196`, `:221`, `:629` and `crates/nvs-host/src/worker.rs:489`. The
  funnel bounds each child, and a parent that left first would break
  `rule:concurrency/nothing-is-still-running-when-a-call-returns`.
- `BlockingJob`, a wait for a job on the blocking pool: `crates/nvs-host/src/blocking.rs:584` and
  `:776`. A job off the core cannot be resumed, so it holds its own instant.
- `DebugStop`, a debugger stop (`crates/nvs-host/src/debug.rs:818`), which suspends the request's
  deadlines by `rule:testing/a-stop-pauses-the-whole-task-tree`.
- `RequestEnd`, the drive of resource destructors at request end, which arms its own instant.

A site that ignores `Cancelled` and parks again would spin past the deadline, so each such site is on
this list or stops at the answer. An upgraded connection's own waits need no argument: its tree states
no `wall_time` (`crates/nvs-runtime/src/ctx/safepoint.rs:284`), and the funnel reads the tree.
**What it spends:** nothing at run time. Safe default: a site whose reason is unclear stays `Parked`.

**`Host::park` and `Host::yield_slice` stay `Parked`, and `block_on` names `ConnectionDrive` alone.**
`crates/nvs-host/src/group.rs:196` is the seam every `Core` member's wait goes through: cache,
channel, pool, socket and event stream. A `Join` there would exempt all of them, which is the
security priority traded away. `:221` is a turn yield that wakes itself, so it is never a long wait.
`block_on_yielding` drives a guest call, which is the request's own wait, so the drive carries its
`Waiting`: `block_on` passes `ConnectionDrive`, and `block_on_yielding` passes `Parked`. The server
tasks the first list found name `ServerTask` too: the queue worker's waits and its `pause`
(`crates/nvs-cli/src/worker.rs`), the exporters' poll (`crates/nvs-cli/src/serve/exporters.rs`) and
the ticker's interval wait (`crates/nvs-server/src/schedule.rs:678`). The event stream's sleep
(`crates/nvs-cli/src/events.rs:230`) is the request's own and stays `Parked`. **What it spends:**
one word per drive. Safe default: unchanged.

**The hand clamps are deleted once the funnel has landed.** `within_wall_time`, `wall_clamp`,
`wall_time_fault` and `wall_time_left` go, with the stored copies: `set_wall`, `send_by`, `wall_or`,
`waited`, `within_wall` and each `wall` field. `wall_bound` stays under the name `off_core_deadline`
for the sites that hand an instant to a blocking-pool job: `crates/nvs-stdlib/src/process.rs:914` and
`:1420`, `crates/nvs-stdlib/src/io.rs:2460` and `:2541`, the SQLite arm of
`crates/nvs-stdlib/src/db/execute.rs:86`, `crates/nvs-runtime/src/capability.rs:634` and the LDAP
lookup at `crates/nvs-stdlib/src/ldap/mod.rs:517`. About two hundred lines go. Usability: a statement
that waits past `wall_time` no longer throws a `DbError` first. Declined: keeping the clamps as a
second layer, because two layers hide a missing one.

**An upgraded connection is published to the watchdog at both doors, and `cpu_time` holds for each
turn.** `Isolate::start` publishes a root connection isolate with `watchdog::publish_here` and gives
it the tree's CPU ceiling with `take_tree_cpu_limit`, when no `watch` is set and a peer or an event
stream is (`crates/nvs-host/src/isolate.rs:790`, `:1161`). A turn ends where the handler waits for
its next input: `Core\Socket::receive`, `Core\Sse::receive` and a sleep inside a connection isolate
(`crates/nvs-runtime/src/peer.rs:621`). The tree's charge starts again there. A count over the whole
connection would close every busy live page after 30 seconds of CPU in 24 hours. `wall_time` stays
off, as b1ce9fe64f decided. No new limit and no new setting. **What it spends:** one publication and
one `Arc<Charge>` per open connection, one clock read per switch into or out of its tree as a request
pays, and two stores per turn. Safe default if a restart of the charge races the sweep: restart only
on the connection's own core, between two of its turns.

**Pings and empty fragments do not restart `[server.connection] idle_timeout`.** `receive` fixes the
idle instant when it starts to wait and keeps it across control frames. A peer that sends only those
is closed with the idle close when the instant passes. `max_lifetime` is compared at the same point,
and in `send`. Usability: a client keeps a quiet connection open by sending a message, and a ping no
longer does. No new setting. Declined: a count of control frames per message, which is a new limit.

**A stream whose reads never block checks its bound every sixteenth read.** `NvsStream::read` and
`write` count the calls that returned bytes since the stream last parked. At each sixteenth it reads
the running tree's stop flags and the stream's filed deadline, and it yields the core when the slice
clock asks (`crates/nvs-host/src/slice.rs:74`). A stop or a passed deadline ends the read with the
timeout error. This covers every protocol client at one place: the notice loops of the database
drivers, the WebSocket codec's fragment loop and any later one. **What it spends:** one add per read,
and one thread-local read, one load and one clock read per sixteen reads that did not park. A read
that parks pays nothing. Safe default if the tree's flags are not reachable from `nvs-host`: compare
the tree's wall bound at the same cadence. Declined: a clock read on every read, and a check in each
driver loop.

**An event stream takes a `max_open` place like a WebSocket.** The door takes `Slot::take` before it
starts the isolate and holds it in the stream's `EventStream` until the body is dropped. With no place
free it answers `503` with `Retry-After` and starts no isolate. **What it spends:** one relaxed add
and sub and one word per stream.

**A resource destructor at request end is bounded, and its outbound request is refused.** The end
drive has no request to send under, so each queued send and read is dropped, and the guest gets the
refusal its `Answer` already promises. The drive itself parks as `RequestEnd` under one instant: the
request's wall deadline, and never sooner than `[limits] fatal_reserve_time` from the drive's start.
A request with no `wall_time` gets `[limits] disconnect_grace`, the bound
`rule:http-server/a-request-outlives-a-client-that-goes-away` already gives such a request. At the
instant the drive drops the future, which frees the instance. A guest call that parks past the wall
deadline ends the same way, with the wall-time `FATAL`. **What it spends:** one timer entry per end
drive that parks. Safe default if `disconnect_grace` is not readable there: `fatal_reserve_time`.

**A blocking-pool job abandoned in the queue is skipped when it is dequeued.** The job reads its slot
before it runs `f`. **What it spends:** one uncontended lock per job on the pool thread. A job that
has started still runs to its end, which the thread bound covers.

**The short-read shortcut reads once more after a hang-up.** The reactor keeps one bit per
registration, set when an event carried a read close or an error. The shortcut is not taken while it
is set. **What it spends:** one branch per reported event and one byte per registration, and nothing
on a keep-alive read with no hang-up.

**Tests wait on events, never on time.** A test sets a deadline that has passed, moves the tree's
`wall_time`, or counts reads, parks and timer entries. No test asserts a wall-clock duration
(`docs/agent/testing.md` § *A test holds on every CI runner*). Each attack ends at its bound with the
error its rule names.

**Lanes:** one lane, in order. Stages 1 to 3 share `scheduler.rs` and the stdlib wait sites. Stage 4
needs stage 2's stop and shares `blocking.rs` with stage 1. Stage 5 needs stage 4's streak check and
shares `socket.rs` with stage 3. Stage 6 shares `block_on.rs` with stage 1 and no file with stages 3
to 5, so it may run beside them once stage 2 has landed.

## Stages

### 1. every wait that is not a request's own names why it outlives wall_time

**Does:** add `Waiting::ParkedPast(PastWallTime)` and move every listed site to it. Nothing is
bounded yet, so no behaviour changes.

**Split:** an `Explore` subagent first lists every caller of `nvs_host::park_until`, `wait_until`,
`Host::park` and `Host::sleep` in `crates/nvs-cli/src` and `crates/nvs-server/src` that runs on a task
serving no request, a queue worker's idle wait among them, as `file.rs:NN` and the enclosing function.
A second lists every park site that ignores `Resumed::Cancelled` and parks again.

- [x] `Waiting` (`crates/nvs-host/src/scheduler.rs:191`) gains `ParkedPast(PastWallTime)`, and the
  scheduler files it as it files `Parked` (`crates/nvs-host/src/scheduler.rs:1186`). `PastWallTime`
  has one documented variant per reason in the standing decision
  (`rule:concurrency/the-parking-contract`).
- [x] `park_until` (`crates/nvs-host/src/timer.rs:251`) and `wait_until`
  (`crates/nvs-host/src/timer.rs:305`) gain a form that takes the reason, for a server task that waits
  on the clock (`rule:concurrency/the-parking-contract`).
- [x] The `nvs-host` sites move to their reason: `Join` at `crates/nvs-host/src/isolate.rs:1002`,
  `crates/nvs-host/src/group.rs:629` and `crates/nvs-host/src/worker.rs:489`; `ServerTask` at
  `crates/nvs-host/src/worker.rs:1273`; `BlockingJob` at `crates/nvs-host/src/blocking.rs:584` and
  `crates/nvs-host/src/blocking.rs:776`; `ConnectionDrive` for `block_on`'s drive
  (`crates/nvs-host/src/block_on.rs:102`); `DebugStop` at `crates/nvs-host/src/debug.rs:818`. The
  standing decision on `Host::park` says why `group.rs:196`, `:221` and `block_on_yielding` stay
  `Parked` (`rule:concurrency/nothing-is-still-running-when-a-call-returns`).
- [x] The server sites move to theirs: `crates/nvs-server/src/serve.rs:1376`,
  `crates/nvs-server/src/serve.rs:3579`, `crates/nvs-server/src/schedule.rs:730`,
  `crates/nvs-server/src/schedule.rs:801`, `crates/nvs-cli/src/serve.rs:1708`, and each site the first
  list names (`rule:errors/on-limit`).
- [x] The request's own waits stay `Parked`: `crates/nvs-host/src/net.rs:1334`,
  `crates/nvs-host/src/net.rs:1462`, `crates/nvs-host/src/channel.rs:332`,
  `crates/nvs-host/src/channel.rs:415`, `crates/nvs-host/src/timer.rs:261`,
  `crates/nvs-host/src/timer.rs:316` and `crates/nvs-server/src/body.rs:378`. Each site the second
  list names stops at `Cancelled` or moves to a reason (`rule:errors/on-limit`).
- [x] New file `crates/nvs-host/tests/park_sites.rs:1` with two tests,
  `every_wait_past_wall_time_names_a_reason_at_a_listed_site` and `only_the_funnel_suspends_a_task`.
  They read every `.rs` file under `crates/*/src`, as `crates/nvs-lsp/tests/stdout_policy.rs:132` reads
  its crates. The first fails naming the `file:line` of a `ParkedPast` whose file and reason are not in
  its table. The second fails on a yielder suspend outside `yield_on` (`rule:errors/on-limit`).

### 2. the park funnel bounds every wait

**Does:** make `yield_on` end a `Parked` wait at the tree's bound, and give that ending the limit's
handler and `FATAL` at `Ctx::cancel` and at `run_helper`.

- [ ] `ArmedStack` answers its turn's bound: the open handler slice's end, or the tree's wall
  deadline, or none (`rule:errors/on-limit`, `crates/nvs-runtime/src/budget.rs:658`,
  `crates/nvs-runtime/src/ctx/safepoint.rs:1033`, `crates/nvs-runtime/src/ctx/safepoint.rs:1046`).
- [ ] `yield_on` reads the bound for `Waiting::Parked`, merges it into the task's one timer entry,
  restores the site's entry on resume, and past the bound claims the wall stop and answers
  `Resumed::Cancelled`. A bound already passed parks nothing (`rule:errors/on-limit`,
  `crates/nvs-host/src/scheduler.rs:1899`, `crates/nvs-host/src/scheduler.rs:1942`,
  `crates/nvs-runtime/src/ctx/safepoint.rs:746`).
- [ ] `Ctx::cancel` under a claimed wall stop gives the handler and the wall-time `FATAL`, and records
  no cancellation. The session reads the poll's branch order first (`rule:errors/on-limit`,
  `crates/nvs-runtime/src/ctx/safepoint.rs:560`, `crates/nvs-runtime/src/ctx/safepoint.rs:1226`).
- [ ] `run_helper`'s error arm reports the wall-time breach for a member that failed past the wall
  deadline, after the memory question (`rule:errors/on-limit`, `crates/nvs-runtime/src/abi.rs:615`,
  `crates/nvs-runtime/src/ctx/limits.rs:1254`).
- [ ] `NvsStream::wait_until_ready` and the connect wait return their error on the funnel's
  `Cancelled` with no change, and the test below pins it (`rule:concurrency/the-parking-contract`,
  `crates/nvs-host/src/net.rs:1471`, `crates/nvs-host/src/net.rs:1334`).
- [ ] Tests in `crates/nvs-host/src/scheduler.rs:1958` and `crates/nvs-host/src/timer.rs:356`:
  `a_park_with_no_deadline_ends_at_its_trees_wall_deadline`,
  `a_park_with_a_later_deadline_ends_at_the_wall_deadline_on_one_timer_entry`,
  `a_named_wait_stays_parked_past_the_wall_deadline`,
  `a_limit_handler_that_parks_is_bounded_by_its_slice_end` and
  `a_park_under_no_wall_time_arms_no_timer_entry`. Each counts timer entries and reads the answer, with
  a deadline already passed or moved by the test (`rule:errors/on-limit`).
- [ ] Tests `a_wait_that_ended_at_the_wall_deadline_runs_the_handler_then_is_fatal` and
  `a_member_that_fails_past_the_wall_deadline_reports_the_wall_time_fatal` in
  `crates/nvs-runtime/src/abi.rs:1` tests (`rule:errors/on-limit`).
- [ ] `rule:errors/on-limit` gains the paragraph: every park ends at the wall deadline in the funnel,
  the one ending, the reasons a wait may outlive it, and what it spends. Its Declined list gains the
  sweep over parked tasks. `bun nv rules --check` is then clean (`docs/rules/errors/on-limit.md:39`).
- [ ] The watchdog's module doc and the doc of `a_core_parked_on_io_past_the_same_wall_time_is_not_flagged`
  say the funnel ends a parked request (`crates/nvs-host/src/watchdog.rs:173`).

### 3. the hand clamps are deleted, crate by crate

**Does:** remove every clamp before a wait and every check after it, so the funnel is the only bound.

**Split:** four writing subagents on the session's model run side by side, one per batch below. Their
files are disjoint and none of them builds. The session then deletes the `Ctx` methods and builds once.

- [ ] Batch 1, network members: `crates/nvs-stdlib/src/net.rs:1257`, `crates/nvs-stdlib/src/net.rs:1389`,
  `crates/nvs-stdlib/src/net.rs:1504`, `crates/nvs-stdlib/src/http.rs:4377`,
  `crates/nvs-stdlib/src/http.rs:4405`, `crates/nvs-stdlib/src/http.rs:4422`,
  `crates/nvs-stdlib/src/http/socket.rs:532`, `crates/nvs-stdlib/src/http/socket.rs:556`,
  `crates/nvs-stdlib/src/mail.rs:1212`, `crates/nvs-stdlib/src/mail.rs:1215`
  (`rule:http-server/one-deadline-covers-the-whole-call`).
- [ ] Batch 2, database: `crates/nvs-stdlib/src/db/pool.rs:170`, `crates/nvs-stdlib/src/db/pool.rs:174`,
  `crates/nvs-stdlib/src/db/open.rs:326`, `crates/nvs-stdlib/src/db/open.rs:836`,
  `crates/nvs-stdlib/src/db/open.rs:1091`, `crates/nvs-stdlib/src/db/transaction.rs:355`,
  `crates/nvs-stdlib/src/db/transaction.rs:361`, and the wire arm of
  `crates/nvs-stdlib/src/db/execute.rs:96` with its comment. The SQLite arm keeps its instant
  (`rule:core-classes/db-statement-members`).
- [ ] Batch 3, stores: `crates/nvs-stdlib/src/cache.rs:1480`, `crates/nvs-stdlib/src/cache.rs:1509`,
  `crates/nvs-stdlib/src/cache.rs:1519`, `crates/nvs-stdlib/src/cache.rs:1796`,
  `crates/nvs-stdlib/src/cache.rs:1840`, `crates/nvs-stdlib/src/cache.rs:2386`,
  `crates/nvs-stdlib/src/cache.rs:2449`, `crates/nvs-stdlib/src/cache.rs:2718`,
  `crates/nvs-stdlib/src/cache.rs:2760`, `crates/nvs-stdlib/src/cache.rs:2799`,
  `crates/nvs-stdlib/src/cache/redis.rs:147`, `crates/nvs-stdlib/src/cache/redis.rs:165`,
  `crates/nvs-stdlib/src/ratelimit.rs:807`, `crates/nvs-stdlib/src/session.rs:681`,
  `crates/nvs-stdlib/src/session.rs:698`, `crates/nvs-stdlib/src/session.rs:715`,
  `crates/nvs-stdlib/src/session.rs:930`, `crates/nvs-stdlib/src/session.rs:1534`,
  `crates/nvs-stdlib/src/session.rs:1578`, `crates/nvs-stdlib/src/session.rs:1622`
  (`rule:errors/on-limit`).
- [ ] Batch 4, connections and the rest: `crates/nvs-stdlib/src/socket.rs:1095`,
  `crates/nvs-stdlib/src/socket.rs:1105`, `crates/nvs-stdlib/src/socket.rs:1210`,
  `crates/nvs-stdlib/src/socket.rs:1242`, `crates/nvs-stdlib/src/sse.rs:885`,
  `crates/nvs-stdlib/src/sse.rs:957`, `crates/nvs-stdlib/src/response.rs:2430`,
  `crates/nvs-stdlib/src/channel.rs:329`, `crates/nvs-stdlib/src/time.rs:3737`, the dial waits at
  `crates/nvs-stdlib/src/ldap/mod.rs:161`, `crates/nvs-stdlib/src/ldap/mod.rs:370` and
  `crates/nvs-stdlib/src/ldap/mod.rs:530`, the after-checks at `crates/nvs-stdlib/src/process.rs:849`,
  `crates/nvs-stdlib/src/io.rs:2464` and `crates/nvs-stdlib/src/io.rs:2543`, and the codec's copy at
  `crates/nvs-server/src/socket.rs:205`, `crates/nvs-server/src/socket.rs:295` and
  `crates/nvs-server/src/socket.rs:523` (`rule:errors/on-limit`).
- [ ] The session's own part, after the batches: `crates/nvs-runtime/src/ctx/late_init.rs:210`,
  `PeerSocket::set_wall` at `crates/nvs-runtime/src/peer.rs:601`, `Emit::send_by` at
  `crates/nvs-runtime/src/stream.rs:195`, and the four methods at
  `crates/nvs-runtime/src/ctx/limits.rs:1048`. `wall_bound` becomes `off_core_deadline`, and the
  off-core sites in the standing decision take the new name. The session confirms whether
  `crates/nvs-stdlib/src/ldap/mod.rs:626` is a lookup or a dial (`rule:errors/on-limit`).
- [ ] Tests that call a deleted method follow: `crates/nvs-runtime/src/ctx/safepoint.rs:2106` and
  `crates/nvs-server/src/serve.rs:5018` (`rule:errors/on-limit`).
- [ ] Test `no_member_clamps_its_wait_to_the_wall_deadline_by_hand` in
  `crates/nvs-host/tests/park_sites.rs:1`. It reads the sources and fails naming the `file:line` of
  each deleted name (`rule:errors/on-limit`).
- [ ] Test `a_statement_wait_that_ends_at_the_wall_deadline_is_the_wall_time_fatal` in
  `crates/nvs-stdlib/src/db/execute.rs:1` tests. The peer never replies, the deadline has passed, and
  the program's `catch` does not run. The fragment says the same
  (`rule:core-classes/db-statement-members`, `docs/rules/core-classes/db-statement-members.md:20`).

### 4. a stream that never runs dry meets its bound, and the pool skips an abandoned job

**Does:** bound a read loop that never parks at the stream itself, read a hang-up that came with a
short read, and skip a job its task gave up on.

- [ ] `NvsStream` counts reads and writes that returned bytes since its last park. At each sixteenth
  it reads the running tree's stop flags and its filed deadline, and yields when the slice clock asks
  (`rule:http-server/time-is-bounded-inside-a-helper`, `crates/nvs-host/src/net.rs:1801`,
  `crates/nvs-host/src/net.rs:1820`, `crates/nvs-host/src/net.rs:1434`,
  `crates/nvs-host/src/slice.rs:296`).
- [ ] `nvs-runtime` answers whether the running turn's tree is stopped for `WALL_TIME` or `CPU_LIMIT`,
  from the tree `TURN` holds (`rule:errors/on-limit`, `crates/nvs-runtime/src/budget.rs:372`).
- [ ] Tests in `crates/nvs-host/src/net.rs:1841`: `a_stream_that_never_runs_dry_ends_at_its_deadline`,
  `a_stream_that_never_runs_dry_ends_when_its_tree_is_stopped`,
  `a_stream_that_never_runs_dry_gives_the_core_to_a_waiting_task` and
  `a_read_that_parks_pays_no_streak_check`. The peer writes until the reader closes. Each counts reads
  or checks at two sizes, and none times a duration (`rule:http-server/time-is-bounded-inside-a-helper`).
- [ ] Test `a_reply_of_endless_notices_ends_at_the_statement_deadline` in `crates/nvs-db/src/pg.rs:1`
  tests. A scripted server sends notices and no reply, and the read ends within sixteen reads of a
  passed deadline. The loops it covers are `crates/nvs-db/src/pg.rs:3160`,
  `crates/nvs-db/src/pg.rs:3612` and `crates/nvs-db/src/pg.rs:1182`
  (`rule:http-server/time-is-bounded-inside-a-helper`).
- [ ] `rule:http-server/time-is-bounded-inside-a-helper` says a stream read that never blocks is
  bounded at the stream, with the cadence and its cost
  (`docs/rules/http-server/time-is-bounded-inside-a-helper.md:1`).
- [ ] The reactor sets a bit on a registration whose event carried a read close or an error, and
  `quiet_after_a_short_read` answers `None` while it is set
  (`rule:concurrency/try-the-syscall-then-park`, `crates/nvs-host/src/reactor.rs:690`,
  `crates/nvs-host/src/net.rs:1613`, `crates/nvs-host/src/net.rs:1573`).
- [ ] Test `a_short_read_that_came_with_a_hang_up_reads_once_more` in `crates/nvs-host/src/net.rs:1841`,
  on Unix. The peer writes and shuts down before the reactor polls, and the test counts one more
  `recv` that returns 0 (`rule:concurrency/try-the-syscall-then-park`).
- [ ] A blocking-pool job reads its slot before `f`. An abandoned one releases its thread, drops its
  wake and returns (`rule:http-server/a-core-is-never-blocked-on-a-syscall`,
  `crates/nvs-host/src/blocking.rs:516`, `crates/nvs-host/src/blocking.rs:572`).
- [ ] Test `a_job_abandoned_in_the_queue_is_skipped_when_it_is_dequeued` in
  `crates/nvs-host/src/blocking.rs:823`. A first job holds the pool's one thread on a channel, a second
  is abandoned in the queue, and a counter shows its body never ran
  (`rule:http-server/a-core-is-never-blocked-on-a-syscall`).

### 5. an upgraded connection is watched, takes a place and is ended by its idle timeout

**Does:** publish a connection isolate at both doors with `cpu_time` counted per turn, give an event
stream a `max_open` place, and stop control frames from restarting `idle_timeout`.

**Split:** an `Explore` subagent lists every `about.md`, reference card and example that says a ping
keeps a connection open or that `cpu_time` covers a whole connection, as `path:NN`.

- [ ] `Isolate::start` publishes a root connection isolate with `publish_here` and calls
  `take_tree_cpu_limit`, where `watch` is `None` and a peer or an event stream is set
  (`rule:concurrency/a-connection-is-a-root-isolate`, `crates/nvs-host/src/isolate.rs:790`,
  `crates/nvs-host/src/isolate.rs:1161`, `crates/nvs-host/src/watchdog.rs:668`).
- [ ] The watchdog restarts the running tree's charge on its own core, and the three turn ends call
  it through `Host` (`rule:concurrency/a-connection-is-a-root-isolate`,
  `crates/nvs-host/src/watchdog.rs:343`, `crates/nvs-stdlib/src/socket.rs:1242`,
  `crates/nvs-stdlib/src/sse.rs:885`, `crates/nvs-runtime/src/peer.rs:621`).
- [ ] Tests in `crates/nvs-server/src/serve.rs:1` tests, on a core registered with a watchdog:
  `a_websocket_handler_that_never_waits_ends_at_its_cpu_limit`,
  `an_event_stream_handler_that_never_waits_ends_at_its_cpu_limit` and
  `a_connections_cpu_count_starts_again_at_each_turn`. The first two read the close the peer gets. The
  third reads the tree's charge after two turns (`rule:concurrency/connection-bounds-are-finite`).
- [ ] The event-stream door takes `Slot::take(bounds.max_open)` before the isolate starts, holds it
  in `EventStream`, and answers `503` with `Retry-After` at the ceiling
  (`rule:concurrency/connection-bounds-are-finite`, `crates/nvs-server/src/serve.rs:2543`,
  `crates/nvs-server/src/bounds.rs:275`, `crates/nvs-server/src/bounds.rs:533`).
- [ ] Tests `an_event_stream_takes_a_max_open_place_and_gives_it_back` and
  `an_event_stream_past_max_open_is_answered_503` in `crates/nvs-server/src/serve.rs:1` tests, against
  a named count as `Slot::take_from` allows (`rule:concurrency/connection-bounds-are-finite`).
- [ ] `Framed::receive_or` fixes the idle instant once per call, keeps it across a ping, a pong and
  an interrupt that was not pending, and compares `expires_at` there and in `send`
  (`rule:concurrency/connection-bounds-are-finite`, `crates/nvs-server/src/socket.rs:400`,
  `crates/nvs-server/src/socket.rs:417`, `crates/nvs-server/src/socket.rs:293`,
  `crates/nvs-server/src/socket.rs:488`).
- [ ] Tests `a_peer_that_sends_only_pings_is_closed_at_the_idle_timeout` and
  `a_peer_that_sends_only_empty_fragments_is_closed_at_the_idle_timeout` in
  `crates/nvs-server/src/socket.rs:1` tests. The idle instant has passed, the peer's frames are queued,
  and the test reads the idle close within sixteen reads (`rule:concurrency/connection-bounds-are-finite`).
- [ ] Two attacks, numbered after the cases already there: a WebSocket handler that never waits at
  `tests/hostile/core/Socket/receive/02-a-handler-that-never-waits.nvs:1` with its peer script, and an
  event-stream handler at `tests/hostile/core/Sse/receive/02-a-handler-that-never-waits.nvs:1`. Each
  ends early with the CPU `FATAL`, and its comments follow AGENTS.md § *Text an end user reads*
  (`rule:testing/feature-proofs`).
- [ ] `rule:concurrency/connection-bounds-are-finite` says an event stream takes a `max_open` place,
  a ping or an empty fragment does not restart the idle timeout, and `cpu_time` is counted per turn.
  Its last sentence changes to a message (`docs/rules/concurrency/connection-bounds-are-finite.md:16`,
  `docs/rules/concurrency/connection-bounds-are-finite.md:52`).
- [ ] `rule:concurrency/a-connection-is-a-root-isolate` says what a turn is and that the count starts
  again at each, with what it spends (`docs/rules/concurrency/a-connection-is-a-root-isolate.md:19`).
- [ ] The `[server.connection]` comment says `idle_timeout` is the longest time with no message
  (`crates/nvs-config/src/default.toml:631`), and each page the list names follows
  (`rule:concurrency/connection-bounds-are-finite`).

### 6. a wait at request end is bounded

**Does:** refuse a destructor's outbound request at request end, bound the end drive by one instant,
and end a guest call that parks past the wall deadline.

- [ ] `Calls::drop` passes a `between` that takes the queued sends and reads and drops them, so each
  `Answer` refuses (`rule:packaging/a-fresh-instance-per-request`,
  `crates/nvs-cli/src/extensions.rs:478`, `crates/nvs-cli/src/extensions.rs:710`,
  `crates/nvs-cli/src/extensions.rs:720`).
- [ ] The end drive takes an instant and parks as `PastWallTime::RequestEnd` with its timer armed.
  Past the instant it drops the future. `Spent` gives the instant: the tree's wall deadline floored at
  `fatal_reserve_time` from now, or `disconnect_grace` with no `wall_time`
  (`rule:errors/on-limit`, `rule:http-server/a-request-outlives-a-client-that-goes-away`,
  `crates/nvs-host/src/block_on.rs:150`, `crates/nvs-host/src/block_on.rs:169`,
  `crates/nvs-cli/src/extensions.rs:468`, `crates/nvs-ext/src/call.rs:1232`).
- [ ] The call drive stops at the funnel's `Cancelled` when the tree is stopped for its wall time: it
  drops the call's future and the member returns `ctx.cancel()`
  It is the one park the second list missed: `block_on_yielding` parks again after a cancellation, so
  past the bound it would poll in a loop until the guest returns
  (`rule:errors/on-limit`, `crates/nvs-cli/src/extensions.rs:876`, `crates/nvs-host/src/block_on.rs:169`).
- [ ] Tests in `crates/nvs-cli/src/extensions.rs:1` tests:
  `a_destructor_that_sends_at_request_end_is_refused_and_the_request_ends`,
  `a_destructor_that_waits_at_request_end_is_dropped_at_its_bound` and
  `a_guest_call_parked_past_the_wall_deadline_ends_with_the_wall_time_fatal`. The guest comes from the
  fixtures `crates/nvs-ext/tests/fixtures.rs:1` builds, or from an inline component where none fits.
  Each asserts the instance place is free again (`rule:packaging/a-fresh-instance-per-request`).
- [ ] `rule:packaging/a-fresh-instance-per-request` says a destructor at request end sends nothing and
  is ended at its bound, with the bound and its cost
  (`docs/rules/packaging/a-fresh-instance-per-request.md:1`).

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is done. `Waiting::ParkedPast(PastWallTime)` exists with six reasons, the clock waits have
`park_until_past` and `wait_until_past`, and every wait that outlives `wall_time` names its reason.
`crates/nvs-host/tests/park_sites.rs` holds each reason to its files and the suspend to `yield_on`,
and both its tests pass. Nothing is bounded yet: `yield_on` still reads no deadline, and the tree
still clamps each wait by hand. Both connection doors still start an unwatched isolate.

The sweep's two reds were already settled on disk. `env-secret-proofs` is not a check in this tree,
and 305c04f660 fixed `a_changed_server_timeout_applies_to_the_next_connection` after the commit the
nightly run tested.

### Next

Stage 2, the park funnel bounds every wait. Its items are under `### 2.` above, with their anchors.
Its files are `crates/nvs-runtime/src/budget.rs:658`, `crates/nvs-runtime/src/ctx/safepoint.rs:560`,
`:746` and `:1226`, `crates/nvs-runtime/src/abi.rs:615`, `crates/nvs-host/src/scheduler.rs:1899` and
`crates/nvs-host/src/timer.rs:356`, and `docs/rules/errors/on-limit.md:39`.

- The funnel reads the bound only for `Waiting::Parked`. `Waiting::ParkedPast(_)` keeps the site's own
  timer entry, and `a_named_wait_stays_parked_past_the_wall_deadline` pins that.
- `Host::park` (`crates/nvs-host/src/group.rs:196`) stays `Parked`, so every `Core` member's wait is
  bounded the moment the funnel lands. The standing decision on `Host::park` says why.
- Expected red after stage 2: a test that parks a task under a passed `wall_time` and expects it to
  wait on, if one exists. `a_core_parked_on_io_past_the_same_wall_time_is_not_flagged`
  (`crates/nvs-host/src/watchdog.rs:173`) is the first to read.

### Backlog

- A host that enumerates no CPU serves requests unwatched (`crates/nvs-cli/src/serve.rs:814`): check it.
- The connection's inbound read caps each `recv` at 8 KiB (`crates/nvs-server/src/io.rs:479`).
