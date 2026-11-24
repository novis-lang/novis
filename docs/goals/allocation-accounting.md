---
looper: 1
title: "the memory accounting in front of each allocation is measured per request, and made one lookup if that pays"
summary: "Reached when a test records how many allocations one served json-api and html-page request makes, the goal's standing decisions hold the per-request cost of the accounting those counts imply, and either the allocator's cache and counters share one thread-local lookup per allocation with a release guard on the saving, or the measured figures show the saving is under 1% of every compare case's CPU and budget.rs records them."
context: [crates/nvs-runtime/src/alloc.rs, docs/rules/programs/memory-priority.md]
stages:
  - id: 1
    title: one served json-api and html-page request each make a recorded number of allocations, and the decision is written
    status: open
    context: [crates/nvs-cli/src/serve.rs, benches/compare/cases/json-api/main.nvs, benches/compare/cases/html-page/main.nvs]
  - id: 2
    title: "only if stage 1 decided it pays: the cache and the counters share one thread-local, looked up once per allocation"
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

# the memory accounting in front of each allocation is measured per request, and made one lookup if that pays

## Why

Every allocation in a release build passes two thread-local lookups. `Pooled` looks up the block
cache (`crates/nvs-runtime/src/alloc.rs:107` `CACHE`, used by `pop` at `:148` and `push` at `:179`).
`Accounting` then looks up the counters (`crates/nvs-runtime/src/budget.rs:359` `COUNTERS`, through
`add` at `:1957`), and `record` and `shift` (`budget.rs:1894`, `:1989`) touch about six cells of it.
`realloc` calls `add` twice, so it looks the counters up twice (`budget.rs:2090`).

Measured in WSL on the development machine, as a release `nvs-runtime` unit test timing one
32-byte round trip through `#[inline(never)]` wrappers, best of 7 batches of 2,000,000, four runs:

| Round trip | Time |
|---|---|
| `Pooled` alone (cache, no accounting) | 2.54–2.82 ns |
| `Pooled` plus `add(+32)` and `add(-32)`, as `Accounting` runs it | 3.52–3.58 ns |
| `System` (glibc) | 4.28–4.47 ns |

The accounting is 0.8–1.0 ns, 21–29% of a counted pooled round trip. That is a large share of an
allocation and a small share of a request. A served `hello` makes 25 allocations
(`crates/nvs-cli/src/serve.rs:6115`, debug build) and costs 14.8 µs of CPU (commit 19abe0fda3's
`bun nv bench-compare --direct` figures), so the whole accounting is about 22 ns, 0.15% of it. For
`json-api` (36.8 µs) and `html-page` (55.5 µs) the same 1% is reached only at about 400 and 600
allocations per request, and nobody has counted those. So this goal counts first and builds only if
the count says it pays (the user's call).

The accounting itself is not in question: it is the per-request memory cap, which
AGENTS.md's priority ordering puts above latency. Only how many lookups it costs is.

## Standing decisions

**Counts decide, not timings (the user's call: measure first).** Stage 1 counts allocations per
served request with the debug count `a_served_request_allocates_its_recorded_count` already uses
(`serve.rs:6131`, `nvs_runtime::budget::allocations()` read per request through
`one_connection_through_the_slot` at `serve.rs:5621`). A debug count is an upper bound on the release
one, since optimization only removes allocations, so a debug count under the threshold settles it.

**The threshold is 1% of a case's recorded CPU per request.** The saving per allocation is taken as
the whole measured accounting, 1.0 ns, which over-states what one lookup less can save. Stage 2 runs
when `allocations × 1.0 ns ≥ 1%` of the case's CPU for `json-api` (368 ns, so ≥ 368 allocations) or
`html-page` (555 ns, so ≥ 555 allocations). Otherwise stage 2's items are each ticked "not taken"
with the counts, and the goal goes to its gate. Do not re-measure the CPU figures: they are 19abe0fda3's.

**Where the decision is written.** The counts, the arithmetic and the verdict go into this file's
`## Standing decisions` as one paragraph and into `budget.rs`'s module doc § *What it spends*
(`budget.rs:191`), which already states the accounting's per-allocation range. Either way the module
doc ends up saying what the accounting costs per request.

**If stage 2 runs: one thread-local, not two.** The cache's classes and the counters move into one
`const`-initialized, `Drop`-free `thread_local!`, so `Accounting::alloc` and `dealloc` look it up once
and hand it to both the free list and `add`'s halves. `alloc.rs`'s § *The one trap* still holds for
the merged one. `realloc` charges its net change through one lookup, raising the mark by the new size
first exactly as today. The cache's bounds (16 classes, 512 blocks each) and every counter's meaning
do not change, and `counting_alloc` (the `cfg(test)` allocator) keeps forwarding to the same shape.
The fallback, if the merge needs `unsafe` the audited blocks do not already cover: keep two
thread-locals and merge only `realloc`'s two lookups.

**If stage 2 runs: the guard is a ratio in `nvs-runtime`.** `Pooled` is `pub(crate)`, so the guard
that times it lives in `alloc.rs`'s tests, release-only
(`#[cfg_attr(debug_assertions, ignore = ...)]`), as the WSL measurement above did: the counted round
trip against `System`'s. Its soft bound is the figure stage 2 reaches in WSL, and its hard bound is
the figure before the change (0.82x of `System`), so losing the merge fails it. Read the spread
across CI runners before trusting a bound: commit 9b2aacda5f is what a bound set on one machine cost.

**The change is kept only if it measures.** Stage 2 times the counted round trip before and after,
five WSL runs each (`bun nv linux-test --release -p nvs-runtime --lib <name> -- --nocapture`). If the
median does not fall by at least 0.3 ns, the change is reverted, the figures are recorded as the
"not taken" case, and the goal goes to its gate.

**No feature moves.** This goal changes no `Core` member, no diagnostic and no page, so it owes no
feature proofs, no `changes.json` entry and no website page.

**What stops it.** One attempt per item and two sessions per stage. A stage 2 that misses its
0.3 ns after one attempt is reverted, not tuned.

**Lanes.** None: stage 2 depends on stage 1's verdict, and both stages are one session each.

## Stages

### 1. one served json-api and html-page request each make a recorded number of allocations, and the decision is written

**Split:** none, one session.

- [ ] `crates/nvs-cli/src/serve.rs:6098`: beside the `hello` case, constants for the `json-api` and
  `html-page` programs read from `benches/compare/cases/json-api/main.nvs:1` and
  `benches/compare/cases/html-page/main.nvs:1` (with `order.json` as `json-api`'s body, sent through
  `Way::served`), and one debug test `a_served_compare_case_allocates_its_recorded_count` that serves
  each case on one keep-alive connection as `a_served_request_allocates_its_recorded_count` does (50
  warm, 200 counted), prints the count per request, and asserts it equals a recorded constant per
  case. No row table: these counts decide a goal, they do not account for each allocation.
- [ ] Run the test once by name: `cargo test --bins a_served_compare_case_allocates_its_recorded_count -- --nocapture`.
- [ ] Write the counts, `count × 1.0 ns` against 368 ns and 555 ns, and the verdict (stage 2 runs or is
  not taken) as one paragraph under `## Standing decisions` above, headed **Stage 1's verdict**.
- [ ] `crates/nvs-runtime/src/budget.rs:191` § *What it spends*: one sentence with the per-request
  cost for `hello`, `json-api` and `html-page` from these counts.

### 2. only if stage 1 decided it pays: the cache and the counters share one thread-local, looked up once per allocation

If stage 1's verdict is "not taken", tick every item below as "not taken, stage 1's verdict" and go to
stage 3.

**Split:** none, one session.

- [ ] Before any edit: add the release-only ratio test in `crates/nvs-runtime/src/alloc.rs:376`'s test
  module, and record five WSL runs of it in the handoff.
- [ ] `crates/nvs-runtime/src/alloc.rs:99` and `crates/nvs-runtime/src/budget.rs:355`: one merged
  `thread_local!`, its doc carrying § *The one trap*, and `alloc.rs`'s and `budget.rs`'s
  § *What it spends* rewritten for one lookup.
- [ ] `crates/nvs-runtime/src/budget.rs:2048`: `alloc`, `alloc_zeroed`, `dealloc` and `realloc` each
  take the merged storage once.
- [ ] Five WSL runs after the change. Keep it only if the median falls by 0.3 ns or more, then set the
  guard's bounds as the standing decision says; otherwise revert and record the figures.
- [ ] Build, clippy, and `cargo test --lib` over `nvs-runtime`'s budget and alloc tests by name, which
  the merge touches.

### 3. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the session that found the cost while fixing the memory-mark guard (commit 9b2aacda5f).
Nothing is built. The per-allocation figures above are measured, and the per-request counts are not.

### Next

Stage 1 from its first item. The check `compare-counts` fails until that test exists.

### Backlog
