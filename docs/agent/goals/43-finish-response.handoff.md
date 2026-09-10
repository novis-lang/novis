# Handoff

## State

**Goal `finish-response` — a response ends where the code says so, and every finally still runs — has
just started; nothing of it has landed yet.** The previous goal's whole acceptance list is this
goal's floor.

**There is no ending between a return and a termination.** `exit` answers
`Err(Fault::Pending(EXITED))` at `crates/nvs-runtime/src/helpers.rs:2591`, and both things that makes
true are deliberate: no `finally` runs, and no `afterResponse` work drains. The drain gates are
`completion.ok && child.has_deferred()` at `crates/nvs-host/src/isolate.rs:665`, the same test again
at `:731`, and `outcome.is_ok()` at `crates/nvs-cli/src/main.rs:1811` — an `EXITED` fails all three,
so the registrations are released unrun at `crates/nvs-runtime/src/ctx/mod.rs:1177-1193`. That is
`rule:concurrency/after-response-outlives-the-connection` working exactly as written. The gap is that
a program with nothing to report and cleanup to run has no other way to stop.

**The keystone is the unwind, and the cheap answer is the wrong one.** Every fallible call site emits
one test — `status != OK`, `crates/nvs-codegen/src/emit.rs:3308` — and its landing block asks one
more: `status == THROWN` takes the handler edge, everything else takes `onward`
(`crates/nvs-codegen/src/emit.rs:3415-3425`), which releases the frame's locals and leaves
(`crates/nvs-ir/src/ir.rs:2585-2592`). A `finally` body lives behind the handler edge. **So a fifth
status inherits `exit`'s mechanics for free and skips every `finally`** — which is the one thing this
goal exists not to ship. It is scheduled on the other line instead: raise it on the throw path
carrying a marker object outside spec § 10's `Throwable` tree, so `finally` runs, `InstanceOf` matches
no arm, and the region's re-propagate carries it to the root. The user settled this; § *Standing
decisions* carries it and the fallback, and neither is re-opened.

**One defect is inherited rather than caused, and it is stage 2.** `Core\Script::onExit` never fires
for a served request. `run_exit_hooks` has three call sites and all three are the CLI's
(`crates/nvs-cli/src/main.rs:1814`, `:1850`, `:1856`); nothing in `nvs-host` drains the queue on the
isolate completion path, so a request's hooks are released unrun with its context — while
`rule:observability/exit-hooks-run-after-the-ladder-before-teardown` writes a paragraph about what
the queue costs *on the request path*. **Verify that reading before writing anything**: it is a grep
over call sites, and a route through a trait object would not appear in it. If the queue is reached
after all, delete the stage and say so here.

**Nothing guards the current behaviour either.** No case in `tests/` registers an `afterResponse` and
then exits — so the "an `exit` runs none of it" half is held by prose alone today. Stage 4's list
adds the guard for it beside the new ending's, which is why
`an_exited_request_still_runs_no_after_response_work` is in a check about the *new* member.

## Next group

**Stage 0 and stage 2 together** — they share no file, and that is the point: stage 0 is a list of
five sentences to correct later (it writes nothing now beyond the record's opening paragraph), and
stage 2 is the precondition everything after it assumes. Neither touches the unwind, so this group
leaves `nvs-ir` and `nvs-codegen` unopened.

- [ ] **Open the record.** One new record, and **do not name a number** — it is claimed by the file
      that lands, one above the highest in `docs/decisions/`. Its opening paragraph is the fourth
      ending: why it sits beside `exit` rather than inside it, and why `finally` running is the line
      that separates the two.
- [ ] **Confirm stage 2's reading.** `run_exit_hooks`'s call sites, and whether anything in
      `nvs-host` reaches the queue by another route. One grep and one read of
      `crates/nvs-host/src/isolate.rs:670-733`.
- [ ] **Drain the exit queue on the served path**, beside the deferred drain at
      `crates/nvs-host/src/isolate.rs:696` and in the order the rule already fixes: hooks, then
      `afterResponse`, then the sweep. The CLI's three arms at `crates/nvs-cli/src/main.rs:1811-1856`
      are the shape to mirror — including which endings get a `Throwable` handed to the hook.
- [ ] **Leave the two silent endings silent.** A `FATAL` and a cancellation run no hook
      (`rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook`), and the cancelled path in
      particular may not park — `crates/nvs-host/src/isolate.rs:784-787` is the existing note on why
      `end_session` is skipped there, and a drain has the same constraint.
- [ ] **Expect no Novis surface in this group.** Stage 2 makes an existing member work on a host it
      never reached; it adds nothing to `Core`.

## Backlog

- **Stage 3, the unwind** — `nvs-ir`, `nvs-codegen`, `nvs-types`, `nvs-diagnostics`. Shares nothing
  with this group, and is the goal's keystone; take it as its own group. Its first read is
  `crates/nvs-ir/src/ir.rs:2585-2592`, before any decision.
- **Stage 4, the member** — `nvs-stdlib`, `nvs-host`, one row in `nvs-stdlib/src/registry.rs`. Cheap
  once stage 3 lands, and shares `crates/nvs-host/src/isolate.rs` with this group.
- **Stage 5, the record and the rulebook** — stage 0's five corrections, and the fragments. Prose
  only, after the behaviour is green.
- When this goal's last check goes green the driver takes goal `gap-zero`.
