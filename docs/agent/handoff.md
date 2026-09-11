# Handoff

## State

**Goal `resource-ceilings` — stage 4 is landed, and stage 5's reader half with it.** A refusal is
now a breach the poll reports: `crate::budget::affords`
(`crates/nvs-runtime/src/budget.rs:@affords`) is the question an allocator asks in front of an
allocation, and a `false` from it records the verdict in a thread-local beside the counters and
raises `SafepointFlags::MEMORY_LIMIT`. `Ctx::over_memory_limit`
(`crates/nvs-runtime/src/ctx/limits.rs:@over_memory_limit`) reads both, because a refused request
holds *less* than its ceiling and every counting reader calls it comfortably inside one.

**The verdict is confined to the request that was refused.** `Ctx` displaces it exactly as it
displaces the armed threshold — `memory_refused_saved`, taken in `Ctx::new` and given back in
`Drop` — so a served core does not answer one `FATAL` per request for ever after the first
refusal. `Ctx::run_limit_handler` takes it for the length of the call, which is the verdict half of
`rule:errors/on-limit`'s reserved slice: a handler entered still carrying the refusal would be
stopped by it at `run_helper`'s question before its first `Core` member.

**`run_limit_handler` now re-arms the threshold at both ends of the reserve it lends**
(`crates/nvs-runtime/src/ctx/hooks.rs:@run_limit_handler`), and `arm_memory_ceiling` is
`pub(super)` for it. Ahead of the next item on purpose: `affords` refuses against the *armed*
number, so a handler given the reserve in `Ctx::memory_limit` alone would be refused every
allocation it needs to write the report with. `handler_isolate` still arms nothing and still fails
open, which that method's doc comment now says.

Nothing under `crates/nvs-runtime/src/string.rs` asks `affords` yet, so the two stage-0 cases that
wait on the refusal are still red. `crates/nvs-runtime/tests/refusal.rs` is where stage 5's cases go.

## Next group

**Stage 5: the value allocators refuse** — one file set, `crates/nvs-runtime/src/string.rs` with
`crates/nvs-runtime/src/object.rs` for the second aborting site. The goal's § *Standing decisions*
settles the three `extern "C"` primitives and nothing above them, so the first item is a decision
before it is an edit. `rule:errors/on-limit` is what the report owes.

- [ ] **`alloc_uninit` goes, and what `NvsStr::build` hands back instead is the decision**
      — `crates/nvs-runtime/src/string.rs:486` is
      `try_alloc_uninit(..).unwrap_or_else(handle_alloc_error)`, and `NvsStr::new`/`from_pieces`
      reach it through `build` (`crates/nvs-runtime/src/string.rs:427`) from most of
      `nvs-stdlib`, so neither can grow an `Option` without a sweep. The immortal empty string is
      the degenerate return stage 5 already names for `nvs_str_concat`, and giving it to `build`
      keeps every signature.
- [ ] **The pre-check, in the one place a string allocation is made** —
      `crates/nvs-runtime/src/string.rs:495`, in front of the `alloc(layout)` at
      `crates/nvs-runtime/src/string.rs:507`, as `if !crate::budget::affords(layout.size())
      { return None; }`. Lands `no_string_allocation_path_can_abort` and the never-allocated half
      of `a_single_operation_past_the_ceiling_never_allocates_what_it_asked_for`, whose sticky
      half is already asserted in `crates/nvs-runtime/tests/refusal.rs`.
- [ ] **The degenerate returns, one primitive at a time** — `nvs_str_append`
      (`crates/nvs-runtime/src/string.rs:1207`) returns its `target` unchanged and
      `nvs_str_concat`/`nvs_str_concat_n` (`crates/nvs-runtime/src/string.rs:1014`,
      `crates/nvs-runtime/src/string.rs:1094`) the immortal empty string, per the goal's
      § *Standing decisions*. `StrWriter::grow` (`crates/nvs-runtime/src/string.rs:813`) is the
      path beside them and aborts today.

## Backlog

- Stage 5's array half — `nvs_array_set` refuses without separating, goal § *Standing decisions*.
- Stage 6 brackets `Core\Cache`'s store; the general property waits for M6's arena (goal prose).
- Stage 7 opens this goal's one record, taking the next free number that day (goal § *Standing
  decisions*).
- `docs/agent/carried-gaps.md` holds what must survive a goal switch.
