# Handoff

## State

**Goal `resource-ceilings` — stage 4 is landed: the allocator publishes.** A request whose ceiling is
computed arms an absolute threshold beside the byte counters
(`crates/nvs-runtime/src/budget.rs:@armed_ceiling`), and `budget::add` compares every *growing*
allocation against it — one thread-local load, then one compare an uncapped request short-circuits
on, the sentinel being `0`. A crossing raises the new `SafepointFlags::MEMORY_LIMIT`
(`crates/nvs-runtime/src/ctx/mod.rs:220`) in the word the request tree polls, through a thread-local
`*const AtomicU64` armed beside the number. `nvs_safepoint` needed no new branch — its existing
`memory_breach` arm is what the flag wakes — and lowers the bit at its foot for a crossing already
given back (`crates/nvs-runtime/src/ctx/safepoint.rs:395`).

**Arming is `Ctx::arm_memory_ceiling` (`crates/nvs-runtime/src/ctx/limits.rs:66`), called by the two
writers that resolve a ceiling** — `refresh_limits` and `set_memory_limit` — so `Core\Config::set`
leaves no stale copy. `Ctx::new` displaces the thread's pair into `memory_ceiling_saved` and `Drop`
arms it again, which is `memory_peak_saved`'s arrangement and what keeps the armed *address* one a
live context holds. A ceiling written any other way (`handler_isolate`, `run_limit_handler`'s
widening) leaves the threshold alone, which fails open: the flag only ever asks for a poll, and the
poll reads `memory_limit` itself.

Stage 4's four named tests are green in `crates/nvs-runtime/tests/allocator_ceiling.rs`.

**The stage-0 loop case is stopped but still red, and the reason is arithmetic, not a missing
line.** `tests/conformance/error/a-loop-that-calls-nothing-is-stopped-by-the-memory-ceiling.nvst`
exists (all three stage-0 `.nvst` cases do) and the loop now dies inside itself — run by hand
against a 16M ceiling it prints `FATAL: … 20986486 bytes held against a ceiling of 16777216`, where
before this session it printed `grew 24 times to 83886080`. What is still missing is its
`--EXPECT-- onLimit memory`: a doubling crosses by about the whole of what it held, the handler's
widened ceiling is at most `[limits] memory` + a quarter of it, so the handler re-breaches at its
first helper call and the zero-retry rule abandons it. **It goes green at stage 5**, where the
allocation is refused and the bytes are never held — and that needs `Ctx::over_memory_limit`
(`crates/nvs-runtime/src/ctx/limits.rs:89`) to answer on a *refusal* as well as on the counter, or a
refused doubling leaves the request under its ceiling and the loop just spins and prints `grew 24
times to 10485760`.

## Next group

**Stage 5: the value allocators refuse** — one file set, `crates/nvs-runtime/src/string.rs` with
`crates/nvs-runtime/src/ctx/limits.rs` for the state a refusal leaves behind. The goal's
§ *Standing decisions* settles the shape: the refusal is a complete no-op, the ctx-less primitives
return a degenerate value and acquire no status, and objects and `nvs_array_new` are out of scope.
`rule:errors/on-limit` is what the report owes.

- [ ] **A refusal is a breach, so the poll that follows one reports it** —
      `crates/nvs-runtime/src/ctx/limits.rs:89`'s `over_memory_limit` is the reader every poll
      shares, and a request that was refused holds *less* than its ceiling. Lands the sticky half of
      `a_single_operation_past_the_ceiling_never_allocates_what_it_asked_for`.
- [ ] **The string allocator pre-checks and `alloc_uninit` goes** —
      `crates/nvs-runtime/src/string.rs:486` is the aborting wrapper stage 5 deletes and
      `crates/nvs-runtime/src/string.rs:495`'s `try_alloc_uninit` the fallible constructor that
      stays. Lands `no_string_allocation_path_can_abort`.
- [ ] **The degenerate returns, one primitive at a time** —
      `crates/nvs-runtime/src/string.rs:1165`'s `nvs_str_append` returns its target unchanged, which
      balances the one reference it consumes, and `crates/nvs-runtime/src/string.rs:1014`'s
      `nvs_str_concat` the immortal empty string. Neither acquires a status, so no call site and no
      lowering changes. Lands `an_append_past_the_ceiling_returns_its_target_unchanged`.

## Backlog

- An isolate is given no memory ceiling at all: `Ctx::isolate`
  (`crates/nvs-runtime/src/ctx/isolate.rs:351`) copies the configuration but no `memory_limit`, and
  `crates/nvs-host/src/isolate.rs:459` never refreshes — so `rule:security/isolate-budget-is-the-trees`
  is unenforced per isolate. Verified by grep, not by a case.
- `run_limit_handler`'s widening (`crates/nvs-runtime/src/ctx/hooks.rs:495`) does not re-arm the
  threshold, so a handler pays a slow-path poll per crossing allocation. Safe, not free.
- Stage 6's detached bracket and stage 7's record are untouched; the record still needs the next free
  ADR number taken at the moment it is written.
- `[context]` gap: the pack's `modules` names only `a-limit-fatal-is-not-catchable.nvst` under
  `tests/conformance/error`, so this session re-derived that the goal's own three stage-0 cases are
  already on disk. Add them to the manifest.
