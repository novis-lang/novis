# Handoff

## State

**ADR 0020 § 1's memory limit is enforced, and the driver's failing check passes.**
`examples/limits.nvs` prints `before the cap`, then
`FATAL: the request exceeded its memory limit — … bytes held against a ceiling of …`, and exits
1. Item 11's three slices are all on disk; the ADR's body is the rule, not this paragraph.

New in `crates/nvs-runtime`: **`budget.rs`** — thread-local live-byte, request and total
counters compiled into every build, behind `budget::Accounting`, which is now the
`#[global_allocator]` for every `not(test)` build and picks `alloc::Pooled` or the platform heap
by exactly the cfg that used to gate registration itself. `counting_alloc::add` feeds the same
counters so a test build counts too. **`Ctx`** gained `memory_base`/`memory_limit` with
`memory_used`, `over_memory_limit`, `memory_breach`, `set_memory_limit` and `refresh_limits`;
`set_config` resolves `[limits] memory` through `nvs_config::Quantity` once and caches it as an
integer, and `Core\Config::set`/`::restore` call `refresh_limits` so the cache cannot go stale.

**Three things to know before touching anything nearby.**

1. **No test binary may install a `#[global_allocator]` any more, and none needs to.** The four
   that did now read `nvs_runtime::budget`'s counters; the playbook's bullet on measuring
   allocations from a test binary owns the mechanism and the exact link error.
2. **Every allocation in every build now costs one thread-local read-modify-write** (three on
   the growth path). That is bought deliberately — AGENTS.md's ordering puts isolation over
   latency — and `budget.rs` § *What it spends* is its only home. The release-only guard in
   `benches/abi-probe/tests/perf_guards.rs:1286` is the instrument if it needs re-measuring;
   its bound is 0.5× and the comment there puts the real ratio an order of magnitude under it.
3. **The breach is noticed at two places, not everywhere it happens** — `run_helper` on the way
   *in* to a `Core` member (before the body, so no `Value` is stranded) and `nvs_safepoint`'s
   slow path. `budget.rs` § *Where the breach is noticed* records the gap that leaves: a loop
   allocating only through `nvs_str_concat`/`nvs_array_append` is not stopped until it calls a
   member, because nothing sets `Ctx`'s flags word for a breach and the safepoint's fast path
   branches on that word. The safe option was taken and the two candidate closures are named
   there, one of which has a same-thread aliasing question under it.

`orient.py`'s pack was accurate apart from the warning it printed itself: `[context] modules`
names `crates/nvs-host/src/budget.rs`, which never existed — the accounting landed in
`crates/nvs-runtime/src/budget.rs`, and that is the pattern the manifest should carry.

## Next group

**Close the memory limit's remaining edges** — the shapes M6's *Verify* names that item 11 did
not reach. File set: `crates/nvs-runtime/src/ctx.rs:265` (`memory_base`/`memory_limit` and
their accessors), `crates/nvs-runtime/src/budget.rs:1` (the counters and the known gap),
`crates/nvs-config/src/tree.rs:156` (`Limits`, where `cpu_time` sits unread beside `memory`)
and `crates/nvs-stdlib/src/config.rs:196` (`Core\Config::set`, which already refreshes it).

- [ ] **`Core\Config::set('memory', …)` above the `[limits]` default takes effect, and above
      `[limits.hard]` returns `false` with the previous value intact** — m6.md § *Verify*, and
      the goal's *Standing decisions* (it returns `false`, it does not throw). The refresh is
      already wired at `crates/nvs-stdlib/src/config.rs:196`; what is missing is a `.nvst` case
      under `tests/conformance/core/` that raises its own ceiling and proves the runtime read
      the new one, and one that proves the hard ceiling refuses.
- [ ] **`Core\Fatal::onLimit` receives the breach** — ADR 0020 § 1. The `LimitReport` the
      handler is given, registered per request beside the pending slot
      (`crates/nvs-runtime/src/ctx.rs:305`, `pending`), running on a reserved slice with zero
      retries. `Core\Fatal` is not in `nvs_stdlib::registry` yet.
- [ ] **The CPU-time limit reaches the same ladder** — ADR 0020 § 1 and
      `crates/nvs-config/src/tree.rs:160` (`cpu_time`). `SafepointFlags::CPU_LIMIT` and its
      message are already in `crates/nvs-runtime/src/ctx.rs`'s `nvs_safepoint`; nothing sets the
      flag, so this is a timer that does, not a new poll site.

## Backlog

- Publish a memory breach into `Ctx`'s safepoint word so a helper-free allocation loop is
  stopped too — `crates/nvs-runtime/src/budget.rs` § *Where the breach is noticed* owns both
  candidate designs and the aliasing question.
- `[context] modules` in `docs/agent/loop-goal.toml` names `crates/nvs-host/src/budget.rs`;
  the file is `crates/nvs-runtime/src/budget.rs`.
- Item 18's `Core\Secret::reveal()` is not in the registry — `docs/implementation-plan.md`.
- `Live::admit`'s same-class check is asked of the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- Item 22's `Core\Script` members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- Stage 5's artifact cache is ADR 0042 as written, and unstarted.
