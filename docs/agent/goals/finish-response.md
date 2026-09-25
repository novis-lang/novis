---
milestone: M7
---
# Loop goal 43 — a response ends where the code says so, and every finally still runs

A program can end its response from anywhere — one member, callable from any frame — and that ending
is an **ordinary** one: every `finally` on the way out runs, the exit queue fires, and
`Core\Task::afterResponse`'s work drains. Today the only way to stop early is `exit`, which runs no
`finally` (`crates/nvs-runtime/src/abi.rs:50-56`), fires the exit queue but drains no deferred work
(`crates/nvs-host/src/isolate.rs:665`), and reports itself as a termination — so the PHP reflex
`echo json_encode($x); exit;` silently drops both halves of a request's cleanup with no diagnostic
anywhere.

The gap is not that `exit` is wrong. `exit` is a *process-termination* keyword
(`rule:statements/exit-is-the-only-termination-keyword`) and its status is what the host is about to
report, which is the whole argument
`rule:concurrency/after-response-outlives-the-connection` makes for running none of the queue over
it. The gap is that **there is no fourth ending** — no way to say "this response is finished" that is
neither a failure nor a termination — so every program that needs one reaches for the termination.

## Why here

After goal `event-streams` because that goal makes a response body two-valued — a whole buffer and a
stream written over time — and *what it means for a response to end* has to be one answer across
both. Deciding it against `Answer`'s single `Option<Bytes>` would settle it for the buffered body and
re-open it the moment a streamed one exists, which is the shape of question this chain pays for by
ordering rather than by revisiting.

Before goal `gap-zero` because this closes a gap that register would otherwise have to carry, and
`gap-zero` can only be emptied once everything that would add to it has run.

What it needs already built: `Core\Task::afterResponse` and its drain (goal `concurrency`), the
served request as a root isolate (goal `server`), and `Core\Script::onExit` with its three endings
(goal `governance`). Nothing after it depends on it.

## Stage 0 — the catch-up

Five places on disk state "three endings" or enumerate the statuses, and each is wrong the moment a
fourth exists. They are corrected in stage 5, not here — this stage is the list, so no session has to
rediscover it:

- `docs/rules/observability/three-endings-fire-the-exit-queue.md` — the title, the three-row table,
  and the closing sentence that already names this goal's extension point: *"A future termination
  kind that should fire the queue is a new case on this queue."*
- `docs/rules/observability/exit-hooks-run-after-the-ladder-before-teardown.md` — *"On the other two
  endings there is no ladder step"*, and the four-mechanism routing paragraph that ends it.
- `docs/rules/concurrency/after-response-outlives-the-connection.md` — *"A request that ended by a
  throw, an `exit` or a `FATAL` runs none of it"*, which stays true and stops being the whole list.
- `crates/nvs-runtime/src/deferred.rs:15-22` — the module doc's own enumeration of the endings that
  run none of it.
- `crates/nvs-runtime/src/abi.rs:50-58` — `EXITED`'s doc, which calls itself the fourth status.

## Stage 1 — the floor

Goal `event-streams`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never
traded.

## Stage 2 — the exit queue drains on the served path at all

**`Core\Script::onExit` never fires for a served request.** `run_exit_hooks` has three call sites and
all three are in `crates/nvs-cli/src/main.rs` (`:1814`, `:1850`, `:1856`); nothing in `nvs-host`
drains the queue on the isolate completion path, so a hook registered by a request is released unrun
with its context. `rule:observability/exit-hooks-run-after-the-ladder-before-teardown` writes a whole
paragraph about what the queue costs *on the request path* — a path it does not currently reach.

This is stage 2 rather than an adjacent bug fix because it is this goal's precondition: the member
stage 4 adds fires the exit queue, and a member whose contract is *"the queue runs"* cannot land on a
host where the queue never runs. The drain belongs beside the deferred one in
`crates/nvs-host/src/isolate.rs:665-696`, in the order
`rule:observability/exit-hooks-run-after-the-ladder-before-teardown` already fixes: hooks first, then
`afterResponse`, then the sweep.

Verify the reading before writing anything — a call site reached through a trait object would not
appear in that grep, and the fix is different if one exists.

## Stage 3 — the keystone: an unwind every `finally` sees and no `catch` admits

The mechanism, and everything after it is mechanical.

What the backend does today: every fallible call site tests `status != OK` and jumps to its landing
block (`crates/nvs-codegen/src/emit.rs:3308`), and the landing block asks exactly one further
question — `status == THROWN` takes the handler edge, **everything else takes `onward`**
(`crates/nvs-codegen/src/emit.rs:3415-3425`), which releases the frame's locals and leaves
(`crates/nvs-ir/src/ir.rs:2585-2592`). A `finally` body lives behind the handler edge. So a **fifth
status is exactly `exit`'s mechanics** — free to add and wrong for this goal, because it skips every
`finally`. Reaching for one is the trap here, and the reason this stage exists at all.

The line to investigate first, and the one this goal is scheduled on: raise it on the **throw** path
carrying a marker object that sits *outside* spec § 10's `Throwable` tree. Then `finally` runs
because the throw path is where `finally` runs; no arm matches because `InstanceOf` against a
`Throwable` subclass is false for a value outside the tree; and the region's re-propagate carries it
to the root frame by frame, running each `finally` on the way. What it costs is one refusal —
`E0780`'s sibling, a `catch` arm that names the marker class — and one widening wherever the pending
value is typed as a `Throwable`.

If that does not hold — if the pending slot cannot carry a non-`Throwable`, or if the unmatched
fall-through cannot preserve the distinction — § *Standing decisions* names the fallback.

## Stage 4 — the member, and the four endings

`Core\Script::finish()`, on `Core\Script` beside `onExit` and **not** on `Core\Response`. The trigger
`rule:concurrency/after-response-outlives-the-connection` defines is host-neutral — the request
task's own frame returning, which under `nvs run` is the end of the script — so the member that
brings that moment forward has to mean one thing on both hosts. `Core\Response::finish()` would mean
nothing under `nvs run` and need a second spelling there.

The classification, which is the whole of the behaviour change:

- The request root treats it as an **ordinary** end. `Completion::ok` stays `true`
  (`crates/nvs-host/src/isolate.rs:747`), so both drain gates pass unedited
  (`:665`, `:731`) and `afterResponse` runs.
- `nvs-server`'s `answer` therefore honours the status the handler declared rather than reaching its
  fail-closed `500` — which is the correct direction here and the opposite of the throw path's, for
  the reason `crates/nvs-stdlib/src/response.rs:73-77` gives: a handler that finished has not failed.
- A fourth `ExitReason` (`crates/nvs-stdlib/src/script.rs:457`), which is the extension
  `rule:observability/three-endings-fire-the-exit-queue` already sanctions. Status `0`, `error` null.
- Inside an exit hook it throws, exactly as `exit` does there
  (`crates/nvs-runtime/src/ctx/hooks.rs:284-285`) and for the same reason:
  `rule:observability/a-hook-observes-and-never-steers`.
- Inside a `Core\Task` child it ends **that child's frame**, not the request. A child is sealed and
  already may not register deferred work (`crates/nvs-runtime/src/deferred.rs:24-34`) or write the
  response (`rule:concurrency/deferred-work-cannot-write-the-response`); a child that could end a
  response it may not write is a hole in both.

## Stage 5 — the record and the rulebook

One new record, and no number written here or anywhere until the file lands. Stage 0's five
corrections, the fragments the record creates and modifies, and the generated reference.

## Standing decisions

- **`finally` runs. This is settled by the user and is not re-opened.** A design that reaches the
  root without running every `finally` on the way is not this feature — it is `exit` with a different
  name, and `exit` already exists. If stage 3's marker-object line proves unworkable, the fallback is
  the *other* half of the same path: keep the value a `Throwable` subclass and exclude it in the
  arm-matching walk, paying one special case in `InstanceOf` instead of one widening. Take the
  fallback and record why in the goal's own record; do not take a fifth status.
- **The member is `Core\Script::finish()`.** Home settled above. If a session finds a reason the name
  collides, rename rather than re-home — the class is the decision, the spelling is not.
- **A tradeoff to state, not to weigh.** Priority 4 pays for priority 2 here: a fourth ending and one
  refusal are new language surface, bought to close the migration trap where the PHP reflex silently
  drops a request's cleanup. AGENTS.md § *Keep each slice small* asks that a feature say what it
  spends; this goal's record is that home. Do not stop to ask whether the surface is worth it.
- **Stage 2 is in scope even though it is a pre-existing defect.** It is this goal's precondition, not
  scope creep. If it turns out the queue *is* reached on the served path by a route the grep missed,
  delete the stage and say so in the handoff — do not invent work for it.
- **Neither `exit`'s semantics nor `rule:errors/escalation-ladder` is re-opened.** `exit` keeps
  running no `finally` and draining no deferred work; a `FATAL` and a cancellation keep running no
  hook. This goal adds an ending beside them and changes none of them.
