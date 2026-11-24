---
looper: 1
title: "the allocator's cache and memory counters share one thread-local lookup per allocation"
summary: "Reached when Accounting's alloc, alloc_zeroed, dealloc and realloc each look up one merged thread-local holding the block cache and the memory counters, a timing probe shows the counted round trip got no slower, a test records how many allocations one served json-api and html-page request makes, and budget.rs's What it spends states the accounting's cost per allocation and per request."
context: [crates/nvs-runtime/src/alloc.rs, docs/rules/programs/memory-priority.md, docs/rules/programs/less-work-is-taken.md]
stages:
  - id: 1
    title: one served json-api and html-page request each make a recorded number of allocations
    status: open
    context: [crates/nvs-cli/src/serve.rs, benches/compare/cases/json-api/main.nvs, benches/compare/cases/html-page/main.nvs]
  - id: 2
    title: the cache and the counters share one thread-local, looked up once per allocation
    status: open
    context: [crates/nvs-runtime/src/budget.rs, crates/nvs-runtime/src/counting_alloc.rs, benches/abi-probe/tests/perf_guards.rs]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: compare-counts
    stage: 1
    name: "one served json-api and one served html-page request each make exactly their recorded number of allocations"
    run: [cargo, test, --bins, a_served_compare_case_allocates_its_recorded_count]
    tests: [a_served_compare_case_allocates_its_recorded_count]
    timeout: 1800
  - id: close
    stage: 3
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, allocation-accounting]
    timeout: 900
---

# the allocator's cache and memory counters share one thread-local lookup per allocation

## Why

Every allocation in a release build passes two thread-local lookups. `Pooled` looks up the block
cache (`crates/nvs-runtime/src/alloc.rs:107` `CACHE`, used by `pop` at `:148` and `push` at `:179`).
`Accounting` then looks up the counters (`crates/nvs-runtime/src/budget.rs:359` `COUNTERS`, through
`add` at `:1957`), and `record` and `shift` (`budget.rs:1894`, `:1989`) touch about six cells of it.
`realloc` calls `add` twice, so it looks the counters up twice (`budget.rs:2090`). One lookup per
allocation does the same work with one lookup less, so `rule:programs/less-work-is-taken` takes it,
whatever a timing shows.

Measured in WSL on the development machine, as a release `nvs-runtime` unit test timing one
32-byte round trip through `#[inline(never)]` wrappers, best of 7 batches of 2,000,000, four runs:

| Round trip | Time |
|---|---|
| `Pooled` alone (cache, no accounting) | 2.54–2.82 ns |
| `Pooled` plus `add(+32)` and `add(-32)`, as `Accounting` runs it | 3.52–3.58 ns |
| `System` (glibc) | 4.28–4.47 ns |

The accounting is 0.8–1.0 ns, 21–29% of a counted pooled round trip. A served `hello` makes 25
allocations (`crates/nvs-cli/src/serve.rs:6115`, debug build) and costs 14.8 µs of CPU (commit
19abe0fda3's `bun nv bench-compare --direct` figures), so the whole accounting is about 22 ns of it.
Nobody has counted `json-api`'s and `html-page`'s allocations, and stage 1 does, so the module doc
can state what the accounting costs per request.

The accounting itself is not in question: it is the per-request memory cap, which
AGENTS.md's priority ordering puts above latency. Only how many lookups it costs is.

## Standing decisions

**The lookup count decides, and the timing only catches a slowdown (the user's call,
`rule:programs/less-work-is-taken`).** The merge is kept once it lands, whatever size of gain the
timing shows: a gain under the noise is still a lookup less. Stage 2 times the counted round trip
before and after, five WSL runs each (`bun nv linux-test --release -p nvs-runtime --lib <name>
--ignored -- --nocapture`), only to catch a merge that made it slower, such as a worse layout of
the merged cells or a lost inline. If the median after is slower than the slowest run before, the
cause is found and fixed in the one attempt. If it cannot be, the change is reverted and the figures
recorded in the handoff.

**One thread-local, not two.** The cache's classes and the counters move into one
`const`-initialized, `Drop`-free `thread_local!`, so `Accounting::alloc` and `dealloc` look it up once
and hand it to both the free list and `add`'s halves. `alloc.rs`'s § *The one trap* still holds for
the merged one. `realloc` charges its net change through one lookup, raising the mark by the new size
first exactly as today. The cache's bounds (16 classes, 512 blocks each) and every counter's meaning
do not change, and `counting_alloc` (the `cfg(test)` allocator) keeps forwarding to the same shape.
The fallback, if the merge needs `unsafe` the audited blocks do not already cover: keep two
thread-locals and merge only `realloc`'s two lookups, which is still less work.

**The timing is a probe, not a guard.** `Pooled` is `pub(crate)`, so the test that times it lives in
`alloc.rs`'s tests, `#[ignore]`d and run by name with `--ignored`, as the WSL measurement above did:
the counted round trip against `System`'s, printed, with no bound. A bound would have to sit past
every CI runner's spread, which is wider than the gain, so it could neither hold the gain nor stay
green (commit 9b2aacda5f is what a bound set on one machine cost). The existing
`an_allocation_round_trip_stays_in_the_pooled_cost_class` in `benches/abi-probe/tests/perf_guards.rs`
keeps catching a change of kind. The one lookup is held by the merged `thread_local!`'s doc, which
says why the cache and the counters share it.

**The per-request counts are a record and a count guard, not a gate.** Stage 1 counts allocations
per served request with what `a_served_request_allocates_its_recorded_count` already uses
(`serve.rs:6131`, `nvs_runtime::budget::allocations()` read per request through
`one_connection_through_the_slot` at `serve.rs:5621`). The test then fails when either case's count
grows, as the `hello` one does. Stage 2 writes the counts into `budget.rs`'s § *What it spends*
(`budget.rs:191`), with the per-allocation range, as the accounting's cost per request.

**No feature moves.** This goal changes no `Core` member, no diagnostic and no page, so it owes no
feature proofs, no `changes.json` entry and no website page.

**What stops it.** One attempt per item and two sessions per stage. A merge that measures slower and
whose cause one attempt does not fix is reverted, not tuned.

**Lanes.** Stages 1 and 2 touch disjoint files (`crates/nvs-cli/src/serve.rs` against
`crates/nvs-runtime/src/alloc.rs` and `budget.rs`) and can run side by side. Stage 2's last item
needs stage 1's counts, so it is taken after stage 1 ends.

## Stages

### 1. one served json-api and html-page request each make a recorded number of allocations

**Split:** none, one session.

- [ ] `crates/nvs-cli/src/serve.rs:6098`: beside the `hello` case, constants for the `json-api` and
  `html-page` programs read from `benches/compare/cases/json-api/main.nvs:1` and
  `benches/compare/cases/html-page/main.nvs:1` (with `order.json` as `json-api`'s body, sent through
  `Way::served`), and one debug test `a_served_compare_case_allocates_its_recorded_count` that serves
  each case on one keep-alive connection as `a_served_request_allocates_its_recorded_count` does (50
  warm, 200 counted), prints the count per request, and asserts it equals a recorded constant per
  case. No row table: these counts record a cost, they do not account for each allocation.
- [ ] Run the test once by name: `cargo test --bins a_served_compare_case_allocates_its_recorded_count -- --nocapture`,
  and write both counts in the handoff.

### 2. the cache and the counters share one thread-local, looked up once per allocation

**Split:** none, one session.

- [ ] Before any edit: add the `#[ignore]`d timing probe in `crates/nvs-runtime/src/alloc.rs:376`'s
  test module, and record five WSL runs of it in the handoff.
- [ ] `crates/nvs-runtime/src/alloc.rs:99` and `crates/nvs-runtime/src/budget.rs:355`: one merged
  `thread_local!`, its doc carrying § *The one trap* and why the cache and the counters share it.
- [ ] `crates/nvs-runtime/src/budget.rs:2048`: `alloc`, `alloc_zeroed`, `dealloc` and `realloc` each
  take the merged storage once.
- [ ] Five WSL runs after the change, recorded beside the five before. Keep it unless the median after
  is slower than the slowest run before.
- [ ] Build, clippy, and `cargo test --lib` over `nvs-runtime`'s budget and alloc tests by name, which
  the merge touches.
- [ ] After stage 1: `crates/nvs-runtime/src/alloc.rs:17` and `crates/nvs-runtime/src/budget.rs:191`
  § *What it spends* rewritten for one lookup, `budget.rs`'s with the accounting's cost per allocation
  and per request for `hello`, `json-api` and `html-page`.

### 3. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the session that found the cost while fixing the memory-mark guard (commit 9b2aacda5f).
Nothing is built. The per-allocation figures above are measured, and the per-request counts are not.

### Next

Stage 1 from its first item, or stage 2's first four items beside it. The check `compare-counts`
fails until stage 1's test exists.

### Backlog
