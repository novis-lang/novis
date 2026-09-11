# Handoff

## State

**Goal `resource-ceilings` — every resource ceiling stops the request that breaks it. Stages 0 and 2
are landed; 3 to 7 are not.** Goal `editor-surfaces`'s whole acceptance list is still this goal's
floor and is untouched.

The word compiled code polls is an `AtomicU64` **outside** `Ctx`, owned by `Ctx::safepoint_word`
(`crates/nvs-runtime/src/ctx/mod.rs:317`) and addressed by the hot slot `SAFEPOINT_OFFSET` still
names. `Ctx::request_safepoint` takes `&self`, so a thread that does not own the request can raise a
bit while the thread that does holds the `&mut` a helper body needs; `Ctx::child` and `Ctx::isolate`
clone the handle, so a tree is stopped by one store. `nvs-codegen` binds the address once in the ABI
entry block, and `nvs_codegen::clif` renders the emitted IR so the two cases that pin that can
assert on it rather than on a machine. **Every bit in that word is now the tree's**, which is why a
cancellation is `Ctx::cancelled`'s own field and no longer a bit — the playbook bullet is the trap.

**What stage 3 still has no way to do is reach the handle from another thread.** Nothing hands it
out and `Ctx` is not `Send`; `crates/nvs-host/src/timer.rs:130`'s `DeadlineView(Arc<Published>)` is
the shape a core already publishes to the watchdog with.

The three stage-0 cases are still red, which is what stage 0 is for, so `python tools/verify.py`'s
conformance step and CI's conformance job are red with exactly those three failures until stages 3,
4 and 5 land, and every run of that tree costs an extra 60 s for the CPU case
(`docs/agent/goals/40-resource-ceilings.md` § *Stage 0*). Nothing is blocked.

`[context] modules` did not print `crates/nvs-host/src/timer.rs`, which holds the publishing shape
stage 3 copies.

## Next group

**Stage 3: the clock the watchdog thread samples** — one file set, `crates/nvs-host/src/watchdog.rs`
and `crates/nvs-host/src/timer.rs`, with one accessor added in
`crates/nvs-runtime/src/ctx/safepoint.rs`. The goal's § *Stage 3* is its items 5 to 7, and the
standing decision above them is that this is CPU time and never wall clock.

- [ ] **Publish the safepoint handle the way a deadline is published** — `Ctx::safepoint_word` at
      `crates/nvs-runtime/src/ctx/mod.rs:317` is an `Arc<AtomicU64>` nothing hands out, so give it a
      view beside `crates/nvs-host/src/timer.rs:130`'s `DeadlineView(Arc<Published>)` and hold it in
      `Watched` at `crates/nvs-host/src/watchdog.rs:116`, which is what one registered core already
      is. `rule:security/isolate-shares-nothing`: the handle a core publishes is the tree root's, and
      one store stops every isolate under it.
- [ ] **Sample the request thread's own CPU clock** — `crates/nvs-host/src/watchdog.rs:349`'s loop
      already wakes on the interval and `crates/nvs-host/src/watchdog.rs:318`'s sweep already reads
      every registered core, so what is new is `GetThreadTimes` on Windows and
      `CLOCK_THREAD_CPUTIME_ID` on Unix, compared against `Ctx::cpu_limit` at
      `crates/nvs-runtime/src/ctx/limits.rs:351`. A platform with no per-thread clock gets no CPU
      ceiling and says so at boot rather than silently getting a wall-clock one.
- [ ] **Raise both halves of the stop** — `SafepointFlags::CPU_LIMIT` through
      `Ctx::request_safepoint` at `crates/nvs-runtime/src/ctx/safepoint.rs:122`, which is what
      compiled code polls, and the deadline through `Ctx::expire_deadline` at
      `crates/nvs-runtime/src/ctx/safepoint.rs:85`, which is what
      `rule:http-server/time-is-bounded-inside-a-helper`'s `bounded_loop` polls inside a member.
      Neither arm of `nvs_safepoint` is new: the CPU one at
      `crates/nvs-runtime/src/ctx/safepoint.rs:220` already runs the tier-1 handler and re-widens the
      reserve, and has only ever been reached by a test. `rule:errors/on-limit`.

## Backlog

- `SafepointFlags::SHUTDOWN` is now shared by a request's tasks, so the first context to poll it
  lowers it for all of them and only one `Core\Signal::onShutdown` runs — harmless today because
  every raiser of that bit is a test (`crates/nvs-stdlib/src/signal.rs:294`), and the fix if a
  delivery lands is to lower it in the context that has the handler.
- Stage 4, the allocator publishing into the word this session relocated —
  `crates/nvs-runtime/src/budget.rs:232` and `docs/agent/goals/40-resource-ceilings.md` § *Stage 4*.
- The three stage-0 cases need no further edit; they go green as stages 3, 4 and 5 land, one each —
  `docs/agent/goals/40-resource-ceilings.md` § *Stage 0*.
- `[limits] cpu_time = "200ms"` in the CPU case is sized for the green day; the 60 s it costs today
  is the case timeout, not the ceiling — `crates/nvs-test/src/run.rs:46`.
- Stage 6's first bracket is `Core\Cache::local`'s `thread_local` — `crates/nvs-stdlib/src/cache.rs`.
- The goal's one new decision record is still unopened; it is owed the refusal, the accounting
  boundary and stage 7's expansion rule — `docs/agent/goals/40-resource-ceilings.md` § *Standing
  decisions*.
