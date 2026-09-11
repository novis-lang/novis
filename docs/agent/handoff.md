# Handoff

## State

**Goal `finish-response` — stage 5's record is open and stage 2's reading is confirmed. No code has
changed yet.** `docs/decisions/0178.md` carries the fourth ending's Context, a three-section Decision
and Consequences; `python tools/records.py --check` is clean. Its `changes:` lists are `[]` on
purpose until stage 5 lands the fragments they will name.

**Stage 2 stands, and its reading is now checked rather than inherited.** `run_exit_hooks` has three
call sites and all three are the CLI's — `crates/nvs-cli/src/main.rs:1951`, `:1987`, `:1993`. The
only other callers of `Ctx::run_exit_hooks` are that module's own cases and
`crates/nvs-runtime/src/ctx/hooks.rs:785`, which is a test, and nothing reaches it through a trait
object. So nothing in `nvs-host` drains the queue, and a request's registrations are released unrun
at `crates/nvs-runtime/src/ctx/mod.rs:1296`.

**Two things the goal file assumes about the fix are wrong, and both change the slice.** `nvs-host`
cannot call `nvs_stdlib::script::run_exit_hooks`: `nvs-stdlib` depends on `nvs-host`
(`crates/nvs-stdlib/Cargo.toml:44`), so that edge cannot run the other way. This repository already
answers the same three-crate problem once — `Session::write_back: fn(&mut Ctx)` at
`crates/nvs-runtime/src/ctx/wiring.rs:51-71`, whose doc names these crates for this reason — and
inverting the seam through `nvs-runtime`, with the one stdlib door filling the pointer, is the shape
to copy.

**The drain does not go beside the deferred one.** It goes inside `finish`
(`crates/nvs-host/src/isolate.rs:897`), after `rule:errors/escalation-ladder`'s climb at `:923-925`
and before `take_buffered_output()` at `:941`: a hook is user code whose output has to cross at the
await like every other byte the child wrote, and an `UncaughtThrow` report needs the `Thrown` that
`:900` takes out of the context. `finish` is also the one classifier both hosts share, so a single
call site covers the task path and `run_here`.

**One question is open, and the next group's first item is it.** `Ctx::pending()` answers
`Option<Cow<'_, str>>` over the pending *exception* (`crates/nvs-runtime/src/ctx/error.rs:361`), so
`failed` at `crates/nvs-host/src/isolate.rs:899` is likely false for an `EXITED`, and the seam's
`outcome` argument (`crates/nvs-stdlib/src/script.rs:450`) has to be derived from something else on
that path.

## Next group

**Stage 2: the exit queue drains on the served path** — one file set:
`crates/nvs-host/src/isolate.rs`, `crates/nvs-runtime/src/ctx/wiring.rs` and
`crates/nvs-stdlib/src/script.rs`. Tag it stage 2 so `[context.stage.2]`'s overlay applies.

- [ ] **Settle how an `exit`ed isolate reaches the classifier**, which fixes the seam's signature:
      read `Ctx::pending`, `Ctx::exit_code` and what `program` leaves behind at
      `crates/nvs-host/src/isolate.rs:899`, against
      `rule:observability/three-endings-fire-the-exit-queue`'s table of reasons.
- [ ] **Add the inverted seam and fill it from the one stdlib door.** A `fn` pointer beside
      `crates/nvs-runtime/src/ctx/wiring.rs:71`, filled by `crates/nvs-stdlib/src/script.rs:450`'s
      `run_exit_hooks`, which stays the only place an ending becomes a report.
- [ ] **Call it from `finish`**, at `crates/nvs-host/src/isolate.rs:926` — after the ladder, before
      the buffer is taken at `:941`, and never on the cancelled path
      (`rule:concurrency/cancellation-runs-no-user-code`). The order is
      `rule:observability/exit-hooks-run-after-the-ladder-before-teardown`'s.
- [ ] **Guard it** beside the sibling drain's own case at `crates/nvs-host/tests/deferred.rs:110`: a
      served request whose hook registered at `Core\Script::onExit` runs before the response ends,
      and a cancelled one that runs none.

## Backlog

- Stage 0's five "three endings" corrections are stage 5's to make — `docs/agent/goals/43-finish-response.md` § *Stage 0*.
- `docs/decisions/0178.md`'s `changes:` lists stay `[]` until stage 5's fragments land.
- Stage 3's keystone: the landing block's `THROWN` test is `crates/nvs-codegen/src/emit.rs:3473`, not the goal file's `:3415-3425`.
- Stage 4's member and the fourth `ExitReason` — `crates/nvs-stdlib/src/script.rs:475`.
- No case in `tests/` registers an `afterResponse` and then exits; stage 4's list adds that guard.
- The stage paragraphs in `docs/agent/goals/43-finish-response.md` still carry the drifted anchors; only `[context] modules`'s comments are corrected.
