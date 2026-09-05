# Handoff

## State

**Goal 6, M7 — ADR 0083 § 7's connection bounds are on disk, and the acceptance check that named
them runs its first test.** `nvs_server::bounds` is the one home of § 7's numbers and of why each
is the number it is; `Connection::default()` is the whole answer rather than a starting point,
because § 7's load-bearing word is *nothing configured*. Six of § 7's eight are fields the framing
layer arms — frame, message, idle, lifetime, send, and § 4's subscriber queue restated from
`nvs_runtime::INBOX_CAP`. The seventh, connections per process, is `bounds::Slot`, a relaxed
process-wide count taken at the `101` and released with the descriptor. The eighth, per tenant, is
not this server's at all: § 7's own parenthetical hands it to ADR 0075's rate limit on the upgrade
request, and that module doc says so.

**`nvs_server::socket::Framed` is what winds the clock.** It holds the bounds, caps every wait by
`Instant::now() + window` against a lifetime computed once, and turns a `TimedOut` into a close
rather than an error — `Closing::Idle` or `Closing::Expired`, with the member answering § 3's
`null`. `send` is the one that still throws, per § 3. `Closing` gained four variants and
`nvs_runtime::peer`'s enum doc is where the code-per-action reasoning lives.

**A connection isolate's end now tells its peer why.** `nvs_host::isolate`'s child body closes the
peer off `Completion::ok` — `Closing::Done` or `Closing::Faulted` (1011) — which is § 1's "closed
with a defined code" for a connection that ran past a `[limits]` budget. Before this, every
connection ended as a bare descriptor drop, so a client read a reset on the normal path too.

**Known gap: none of § 7's six has a `[server]` key**, so changing one is a rebuild.
`nvs_server::bounds`' § *Known gap* names where the keys belong. **The wake seam is unchanged and
still open** — `nvs_runtime::Ctx::deliver`'s known gap at `crates/nvs-runtime/src/ctx/isolate.rs:72`
states it.

## Next group

**The rest of the § 7 check's test list, all three over `crates/nvs-server/src/serve.rs`'s test
module and the isolate teardown it drives.** Take 1 first: it is the test for the mechanism this
session landed, and the fixture it needs is the one items 2 and 3 also want.

- [ ] **`a_connection_over_its_budget_is_closed_with_the_defined_code_not_oom`** — ADR 0083 § 1's
      "closed with a defined code". The close is landed at
      `crates/nvs-host/src/isolate.rs:625`, keyed off `Completion::ok`, which `finish` reads from
      `Ctx::pending()` at `crates/nvs-host/src/isolate.rs:730`. What is missing is an induction: the
      fixture's connection program is a plain `fn(&mut Ctx) -> String` at
      `crates/nvs-server/src/serve.rs:1963`, so it trips no Novis allocation site.
      `crates/nvs-runtime/src/ctx/limits.rs:46`'s `set_memory_limit` is the ceiling to set, and the
      playbook's `Ctx::pending_slot` bullet is the warning that reading a pending throw needs an
      exception class table installed first. Assert the client reads a **close frame carrying
      1011** — the socket-pair shape is at `crates/nvs-server/src/serve.rs:2407`.
- [ ] **`a_connection_whose_isolate_panics_is_contained`** — the same seam one step further out:
      `Ended`'s `Drop` at `crates/nvs-host/src/isolate.rs:589` fires on a forced unwind, but the
      close added at `crates/nvs-host/src/isolate.rs:625` sits *after* `finish` and so does not run
      for a panic. Decide whether the close moves into `Ended` (which would need the peer reachable
      from there) or whether a panicking isolate is deliberately a reset; either way the module doc
      is the home of the answer.
- [ ] **`reload_and_shutdown_close_every_connection_after_the_drain`** — ADR 0083 § 7's third
      bullet. The drain already answers the probe and `isDraining()`; what it does not do is close
      open *connections* with a code after it. `crates/nvs-server/src/serve.rs:1260`'s
      `serve_on_this_core` is where the drain is decided, and `bounds::Slot`'s count is the only
      thing that currently knows how many connections are open.

## Backlog

- The wake seam: a park over both sources — `crates/nvs-runtime/src/ctx/isolate.rs:72` states it,
  and ADR 0083 § 3 specifies it.
- `[server]` keys for § 7's six bounds — `nvs_server::bounds`' § *Known gap*.
- § 4's `slow_subscribers_closed` becomes an ADR 0076 series — `crates/nvs-runtime/src/peer.rs:169`.
- The § 7 hot-reload pair, `an_open_connection_keeps_its_compiled_unit_across_an_edit` and
  `a_connection_opened_after_the_swap_runs_the_new_unit` — ADR 0017 owns the swap.
