# Handoff

## State

**Goal 6, M7 — ADR 0083 § 1's connection isolate starts.** `serve_connection` takes the
slot's other half after the request has joined and starts
`Isolate::new(program, args, Output::Capture)` from the **connection's** own context
(`crates/nvs-server/src/serve.rs:806`), keeping the handle beside `ctx` and joining it after
`hyper`'s connection future ends (`crates/nvs-server/src/serve.rs:854`). **No `101`, decided
rather than deferred**: nothing can frame a byte yet, so the upgrading request answers its own
ordinary response and the status, the `OnUpgrade` and the socket hand-over are the framing
slice's — that function's doc comment is the home of the reasoning. **The member still throws**
(`crates/nvs-stdlib/src/socket.rs:236`).

**§ 1's third test is not writable, and the reason is a leak on the request path.**
`nvs_host::Scheduler` keeps every finished task's whole `Ctx`
(`crates/nvs-host/src/scheduler.rs:881`) and nothing under a server drains it, so a served
request's carrier and arena live as long as the process — O(requests served), which is ADR
0004's own definition of a leak. `docs/agent/playbook.md` § *Writing a test case* owns the
diagnosis and the tell; `crates/nvs-server/src/serve.rs:1875` says why the case is absent.

**§ 5's check needs a decision before its test can exist.** The offer is gated on `hyper`
leaving an `OnUpgrade`, and SSE is not an HTTP upgrade — a `Core\Sse::upgrade` request is an
ordinary `200` — so `sse_is_a_connection_isolate_with_no_receive` cannot reach a slot on
today's door.

## Next group

**The retention first, then § 1's last test over it.** Item 1's file set is
`crates/nvs-host/src/scheduler.rs` and `crates/nvs-cli/`; items 2 and 3 are
`crates/nvs-server/src/serve.rs` and `crates/nvs-runtime/src/ctx/inbound.rs`. Item 1 is first
because it is a leak, and because two of this goal's checks cannot be honest until it lands.

- [ ] **A finished task's context is not kept for a task nobody will collect** — `Finished`
      carries the `Ctx` a request boundary reads back (`crates/nvs-host/src/scheduler.rs:542`),
      and the `Return` arm pushes one per task (`crates/nvs-host/src/scheduler.rs:881`) onto a
      list only `take_finished` empties (`crates/nvs-host/src/scheduler.rs:1012`). The two
      production consumers both `find(|f| f.id == root)` a single **root** task
      (`crates/nvs-cli/src/main.rs:1249`, `crates/nvs-cli/src/runner.rs:356`), and a child's
      answer is delivered through whatever spawned it — `Isolate`'s own slot
      (`crates/nvs-host/src/isolate.rs:392`) — never through this list, so a child's context
      buys nothing by being held. The ids must stay: `run_until_idle` reads them for ADR 0115
      § 2's reactor deregistration (`crates/nvs-host/src/reactor.rs:759`). Decide and record
      whether the `Ctx` becomes optional or the list is scoped to roots; say what it spends in
      the `Finished` doc either way.
- [ ] **§ 1's arena test, once the retention is fixed** — write
      `the_upgrading_requests_arena_is_released_while_the_connection_is_open` where the comment
      now stands (`crates/nvs-server/src/serve.rs:1875`): `upgrade_leaving` already puts two
      readings of `nvs_runtime::budget::live_bytes` into `said`
      (`crates/nvs-server/src/serve.rs:1762`), and `upgrade_once`
      (`crates/nvs-server/src/serve.rs:1789`) is the whole fixture. Two mebibytes on the
      request's carrier, and assert the drop rather than that the connection ran.
- [ ] **The member fills the slot** — both of ADR 0083 § 2's entry forms into one
      `nvs_runtime::Program` (`crates/nvs-stdlib/src/socket.rs:236`), left in the slot the door
      offers (`crates/nvs-runtime/src/ctx/inbound.rs:667`). `crates/nvs-server/src/serve.rs:1725`
      is the hand-written `Upgrade` the tests fill it with, which is the shape the member owes.
- [ ] **§ 5's SSE decision, then its test** — the offer is `hyper`'s `OnUpgrade` and nothing
      else (`crates/nvs-server/src/serve.rs:697`), so an SSE request — an ordinary `200`, no
      protocol upgrade — reaches its program with no slot. Either the slot is offered to every
      request and *what the door may do with it* is what `OnUpgrade` gates
      (`crates/nvs-runtime/src/ctx/inbound.rs:657` would carry the flag), or § 5 gets a second
      door path. The first keeps one mechanism, which is § 1's whole argument; it also rewrites
      `only_an_upgradable_request_is_offered_a_slot`
      (`crates/nvs-server/src/serve.rs:1655`), so decide it before writing either.

## Backlog

- `Core\Socket`'s capability row and gap list — `crates/nvs-stdlib/src/socket.rs`'s module doc.
- The `101`, `tungstenite` framing and the socket hand-over — ADR 0083 §§ 3-4, and
  `serve_connection`'s doc names what it is holding for them.
- A pipelined request behind an upgraded one is undefined at the door —
  `crates/nvs-server/src/serve.rs:601`.
