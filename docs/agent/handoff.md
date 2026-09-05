# Handoff

## State

**Goal 6, M7 — ADR 0083 § 7's acceptance check runs three of its six tests.** On disk:
`every_connection_bound_is_finite_with_nothing_configured` at `crates/nvs-server/src/bounds.rs:197`,
and now `a_connection_over_its_budget_is_closed_with_the_defined_code_not_oom` and
`a_connection_whose_isolate_panics_is_contained`, both in `crates/nvs-server/src/serve.rs`'s test
module. **The previous handoff called the remainder "all three" and it is three *others*:** the
check's two ADR 0017 tests were never landed either, so the group below is the whole rest of it.

**A connection over its `[limits]` budget ends 1011, and no mechanism was added for it.** The close
was already keyed off `Completion::ok`; what was missing was an induction a hand-written program can
perform. The fixture sets its own ceiling (`Ctx::set_memory_limit`), allocates past it, and then
makes the two calls `nvs_runtime`'s safepoint poll makes on a memory breach — `Ctx::memory_breach`
and `Ctx::set_pending` — because a `fn(&mut Ctx) -> String` has no safepoint between two statements
to make them for it. It releases the hog before reporting, or the teardown breaches again inside
`run_helper`.

**A panicking connection isolate is deliberately a reset**, and that decision now lives in the
comment on the close branch in `nvs_host::isolate`. `nvs_runtime::run_task` contains the panic, but
the unwind leaves through `Ended`'s guard without reaching the close, and moving the close into that
guard is refused twice over: the guard holds no context to reach a peer through, and a close is a
*write*, which parks — where a stack being unwound may not.

**Known gap: none of § 7's six bounds has a `[server]` key**, so changing one is a rebuild;
`nvs_server::bounds`' § *Known gap* names where the keys belong. The wake seam is unchanged and
still open — `nvs_runtime::Ctx::deliver`'s known gap states it.

**`orient.py` did not print `crates/nvs-host/src/scheduler.rs`**, which is where "is a panic
contained" is actually answered; `[context] modules` wants a `nvs-host/src/scheduler.rs` pattern.

## Next group

**The three tests § 7's check still names. One is the drain and two are ADR 0017's swap; the file
set is `crates/nvs-server/src/serve.rs`, `crates/nvs-server/src/bounds.rs` and
`crates/nvs-server/Cargo.toml`.** Take the drain first — it is the only one whose mechanism is
already in this crate.

- [ ] **`reload_and_shutdown_close_every_connection_after_the_drain`** — ADR 0083 § 7's third
      bullet. The drain begins at `crates/nvs-server/src/serve.rs:1371` and parks until `outstanding`
      reaches zero at `crates/nvs-server/src/serve.rs:1379`; what it never does is *close* what is
      still open, so a shutdown ends every connection as the reset the panic case above pinned.
      `crates/nvs-server/src/bounds.rs:132`'s `OPEN` is the only thing that knows how many there
      are, and `crates/nvs-server/src/socket.rs:343`'s `Framed::close` is what a drain would have to
      reach — which the accept loop cannot, because the peer moved into the isolate at
      `crates/nvs-server/src/serve.rs:1016`. Decide whether the drain cancels the connection
      isolates or whether a `Closing` reaches them another way, and record it where the close lives.
- [ ] **`an_open_connection_keeps_its_compiled_unit_across_an_edit`** — ADR 0017, and triage before
      writing a line: `crates/nvs-server/Cargo.toml:1` names `hyper`, `nvs-host`, `nvs-config` and
      `nvs-runtime` and nothing that compiles or caches a unit, so the check's `-p nvs-server` may be
      the playbook's misfiling rather than an unlanded slice.
- [ ] **`a_connection_opened_after_the_swap_runs_the_new_unit`** — the same triage over
      `crates/nvs-server/Cargo.toml:1` and the same file set; the two are one fixture if either can
      be hosted here at all.

## Backlog

- § 7's six bounds have no `[server]` key — `crates/nvs-server/src/bounds.rs` § *Known gap*.
- The wake seam — `nvs_runtime::Ctx::deliver`'s known gap.
- ADR 0083 § 5's event stream has no `200 text/event-stream` response half —
  `sse_is_a_connection_isolate_with_no_receive`'s own doc comment says so.
