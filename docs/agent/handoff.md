# Handoff

## State

**Goal 6, M7 — ADR 0083 § 7's acceptance check runs four of its six tests.** On disk:
`every_connection_bound_is_finite_with_nothing_configured` at `crates/nvs-server/src/bounds.rs:213`,
`a_connection_over_its_budget_is_closed_with_the_defined_code_not_oom`,
`a_connection_whose_isolate_panics_is_contained` and now
`reload_and_shutdown_close_every_connection_after_the_drain`, the last three in
`crates/nvs-server/src/serve.rs`'s test module.

**The drain's close is the connection's own, taken at its next wait.** Neither way of reaching one
from outside survives: cancelling the isolate tears its task down at its next safepoint, which is
the reset § 7 asks to replace, and the socket moved into the isolate at the `101` so there is no
second handle to write a frame through. So the drain is one more instant in `Framed::arm`'s minimum
and `Closing::ShuttingDown` — 1001, new in `crates/nvs-runtime/src/peer.rs` — is one more answer in
`Framed::expiry`'s. `crates/nvs-server/src/socket.rs`'s `receive` is the whole argument, including
why the period runs from when a connection *sees* the drain. The period is `Connection::drain`,
1 second, `crates/nvs-server/src/bounds.rs`.

**The remaining two tests are ADR 0017's and `-p nvs-server` cannot host them.** That crate has no
compiled unit at all: `crates/nvs-server/Cargo.toml` names `hyper`, `nvs-host`, `nvs-config`,
`nvs-runtime` and `jiff`, its door takes a handler closure, and the per-mount unit and its swap live
in `crates/nvs-cli/src/serve.rs` (module doc line 57, "one compiled unit per mounted entry"). The
check's `args` is what is wrong, and the group below is that repair plus the two tests.

**Known gap: a connection already parked on a read when the drain begins does not see it until that
read ends**, which is `Connection::idle` away at worst — the same wake seam a topic delivery needs
(`nvs_runtime::Ctx::deliver`'s known gap), because a parked `Read` ends on its deadline, on
readiness or on a cancellation and on nothing else. `socket.rs`'s `receive` states it.

**`orient.py` printed ADR 0083 §§ 1-4 but not § 7**, which is the section this item implements, and
still does not print `crates/nvs-host/src/scheduler.rs`, where cancellation's "dies at its next
safepoint" is stated. `[context] adrs` wants `0083 § 7`; `[context] modules` wants
`nvs-host/src/scheduler.rs`.

## Next group

**The check's last two tests, and the filing repair they need first. The file set is
`docs/agent/loop-goal.toml`, `crates/nvs-cli/src/serve.rs` and `crates/nvs-cli/Cargo.toml`.** Take
the triage first: it decides where the other two are written.

- [ ] **Split ADR 0083 § 7's check at `docs/agent/loop-goal.toml:3817`** so the two ADR 0017 tests
      run where a compiled unit exists. Read `crates/nvs-cli/src/serve.rs:57` (the module doc's
      "one compiled unit per mounted entry, held for the life of the process"),
      `crates/nvs-cli/src/serve.rs:294` (the resolve that is a cache hit on the unit) and
      `crates/nvs-cli/src/serve.rs:347` ("both halves of what the selected unit is") before deciding;
      `crates/nvs-server/Cargo.toml:12` is the manifest that rules the current filing out. Leave the
      four landed names on the `-p nvs-server` check and give the two a second `[[check]]`.
- [ ] **`an_open_connection_keeps_its_compiled_unit_across_an_edit`** — ADR 0083 § 7's second
      bullet over ADR 0017. A connection isolate started against one unit runs on it after the
      pointer has been swapped; the swap is `crates/nvs-cli/src/serve.rs:294`'s resolve and the
      isolate is started from `crates/nvs-server/src/serve.rs:1030`, which takes the program it was
      handed and never re-resolves.
- [ ] **`a_connection_opened_after_the_swap_runs_the_new_unit`** — the other half, over the fixture
      the item above builds: the second connection resolves again at
      `crates/nvs-cli/src/serve.rs:294` and reaches
      `crates/nvs-server/src/serve.rs:1030` with the *new* program, so it is that slice's second
      assertion rather than a second set-up.

## Backlog

- None of § 7's seven bounds has a `[server]` key — `crates/nvs-server/src/bounds.rs`'s § *Known gap*
  names where they belong.
- The wake seam: one slice closes both the drain's reach into a parked read and a topic delivery's
  (`nvs_runtime::Ctx::deliver`'s known gap).
- ADR 0083 § 5's `200 text/event-stream` response head is still unwritten — `serve_connection`'s doc
  says so.
