---
looper: 1
title: "an allocation reads its memory counters in one lookup, and a timing warning prints its figures"
summary: "an allocation and a free each look up the memory counters once, every soft-bound warning prints the times behind its ratio, and the two allocator guards' figures are recorded before and after on Windows and Linux"
context: [crates/nvs-runtime/src/budget.rs, benches/abi-probe/tests/perf_guards.rs, docs/rules/testing/perf-two-mechanisms.md]
stages:
  - id: 1
    title: "every soft-bound warning prints the times behind its figure, and linux-test runs a cost guard"
    status: done
    context: [crates/nvs-lsp/tests/latency.rs, crates/nvs-cli/src/testing.rs, tools/nv/cmd/linux-test.ts, tools/nv/test/linux-test.test.ts]
  - id: 2
    title: an allocation and a free each look up the memory counters once
    status: done
    context: [crates/nvs-runtime/src/budget/gauge.rs, docs/rules/http-server/admission-is-arithmetic-not-a-number.md, docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md, docs/rules/observability/a-memory-peak-is-recorded-not-asked-for.md]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: linux-test-cost-guard
    stage: 1
    name: linux-test runs a release cost guard with -p and still refuses a debug -p
    run: [bun, test, tools/nv/test/linux-test.test.ts]
    tests: ["a release run of one package's test binary is accepted"]
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, allocation-counters]
    reach: true
    floor: false
---

# an allocation reads its memory counters in one lookup, and a timing warning prints its figures

## Why

CI's cost guards warn on most runs about two ratios against the platform heap:

| Guard | Soft / hard | Typical warning |
|---|---|---|
| `the_memory_mark_stays_small_beside_the_allocation_it_fronts` (`benches/abi-probe/tests/perf_guards.rs:2849`) | 0.25x / 1.0x | linux-x86_64, 0.36x to 0.55x, steady |
| `an_allocation_round_trip_stays_in_the_pooled_cost_class` (`benches/abi-probe/tests/perf_guards.rs:2772`) | 1.0x / 2.0x | macos-aarch64, 1.05x to 1.16x |

The cause is in the allocation path, not the guards. `Accounting::alloc` runs `add` → `record` +
`carry` (`crates/nvs-runtime/src/budget.rs:1437`), and each counter is its own `thread_local!`:
`DETACHING`, `LIVE`, `gauge::LIVE_MARK`, `PEAK`, `CEILING`, `REQUESTS` and `TOTAL` on an
allocation, and `DETACHING`, `LIVE` and `gauge::LIVE_MARK` again on a free. That is ten thread-local
lookups per round trip, plus the pool's `CACHE`. On macOS each lookup is a call through the
thread-local descriptor, so the counting costs about what the pool saves. On Linux each lookup is
cheap, but glibc's small-block cache makes the heap round trip so short that ten load-add-store
steps are a third of it. Every allocation of every request pays this.

The warnings also cannot be read on their own. The times behind a ratio go through `println!`, which
libtest holds back on a passing test, so a CI annotation says `0.362x` and nothing about which side
moved.

## Standing decisions

**One thread-local for the allocation path's counters.** A `#[repr(C)]` struct of `Cell`s in one
`const`-initialized `thread_local!` in `budget.rs`, holding no `Drop` type (`crate::alloc`'s *one
trap*), replaces `LIVE`, `PEAK`, `CEILING`, `REQUESTS`, `TOTAL`, `DETACHING`, `DETACHED` and
`gauge`'s `LIVE_MARK` and `DETACHED_MARK`. `add` and `carry` take it once and pass it down, so
`record` and `gauge::far_from` take the struct or the mark's value, not a `LocalKey`. The fields an
allocation reads on every call come first, and a `const` assertion holds them inside the first 64
bytes; `DETACHED` and `DETACHED_MARK`, read only inside a `Detached` bracket, follow. Every other
cell stays where it is: `TURN`, `POLLED`, the refusal bit, `IDLE`, `IDLE_MARK`, `SHARES` and `GAUGE`
are off the per-allocation path.

**The struct is not over-aligned.** Its hot fields span 49 bytes from an 8-aligned start, so they may
cross a cache line. `#[repr(align(64))]` on a thread-local is held to the platform loader's TLS
alignment, which this tree has never tested on Windows, Miri or ASAN, and Linux's figures show no
cost to buy back. The `const` assertion holds the hot fields inside 64 bytes, so a later `align(64)`
is one attribute.

**The marks move into `budget`.** `gauge::report` already reads `budget`'s `LIVE` and `DETACHED`, so
the two modules share this state today. Keeping the marks in `gauge` would cost a second lookup per
call for no separation that exists. `gauge`'s module doc says the marks it compares against live in
`budget`'s counter struct.

**A warning carries its own evidence.** In each of the three judges `rule:testing/perf-two-mechanisms`
names, the soft-bound message carries what that attempt's `println!` line carries: the figure, its
detail (the two times and their `yd` ratios) and the attempt marker (attempt, load, both yardstick
readings). `crates/nvs-lsp/tests/latency.rs:212` and `crates/nvs-cli/src/testing.rs:119` follow
`perf_guards.rs`'s `judge` (`benches/abi-probe/tests/perf_guards.rs:283`). The rule's sentence on
the warning says so.

**No bound changes in this goal.** Both soft bounds stay. Stage 2 measures before and after on this
machine, Windows and WSL. A bound chosen from one machine's figures is not a fix
(`rule:testing/perf-two-mechanisms`'s Declined list). If a steady CI run still warns after the goal
end's push, the warning now prints its times, and that is the evidence a bound change starts from.
It goes to `### Backlog`, not into this goal.

**`bun nv linux-test` runs a release cost guard.** It refuses `-p` today, because a debug `-p`
writes a second copy of the tree. `--release -p` is the cost guards' exception (AGENTS.md § *The
rules only this project has*), so `refusal` accepts `-p` when `--release` is in the arguments, and
refuses it otherwise, as now. Its `USAGE` names the release form.

**Lanes.** Stage 1's files and stage 2's files are disjoint. Stage 2's before-and-after measurement on
Linux needs stage 1's `linux-test` change, so stage 1 runs first. `memory-overshoot-net`'s stage 1
edits `budget.rs`'s module doc (its anchor `crates/nvs-runtime/src/budget.rs:233`). This goal is
ahead of it on the chain, and its anchor moves with this goal's edit.

**Trade-offs.**
- Security: none. The cap, the mark and the ceiling check read and write the same values in the same
  order. The count-based guards and the budget tests prove it.
- Latency: on macOS an allocation round trip goes from about ten thread-local calls to two. On
  Windows the guards' update went from 1.62 ns to 1.36 ns and the pooled round trip from 4.47 ns to
  3.88 ns. On Linux nothing changes: `budget::carry`'s disassembly before and after is the same
  instructions, one `%fs` offset per cell, because a local-exec thread-local already costs no lookup.
- Memory: none per request or per task. Per thread, the same cells and at most a few bytes of padding.
- Simplicity: one struct replaces nine cells, and each allocation shows in one place what it touches.
  A warning in a CI log needs no rerun to read.

## Stages

### 1. every soft-bound warning prints the times behind its figure, and linux-test runs a cost guard

**Does:** a CI annotation from any timing judge shows both times, their yardstick ratios and the load,
and `bun nv linux-test` can run the two allocator guards in release under WSL.

- [x] `perf_guards.rs`'s `judge_where_measurable` passes `reading.detail` and `marker` into the `warn`
  message. `benches/abi-probe/tests/perf_guards.rs:283`
- [x] `latency.rs`'s `judge` adds the attempt and both yardstick readings to its warning.
  `crates/nvs-lsp/tests/latency.rs:212`
- [x] `testing.rs`'s `judge_margin` adds `detail` and the attempt marker to its warning.
  `crates/nvs-cli/src/testing.rs:119`
- [x] The rule's sentence on the soft-bound warning says the warning carries the times, the detail
  and the attempt marker. `docs/rules/testing/perf-two-mechanisms.md:14`
- [x] `linux-test`'s `refusal` accepts `-p` beside `--release`, its `USAGE` and header name the
  release form. `tools/nv/cmd/linux-test.ts:37`, `tools/nv/cmd/linux-test.ts:28`
- [x] The test `a release run of one package's test binary is accepted`, beside the existing `-p`
  refusals. Run once by name: `bun test tools/nv/test/linux-test.test.ts`. `tools/nv/test/linux-test.test.ts:15`

### 2. an allocation and a free each look up the memory counters once

**Does:** the standing decision's one struct replaces the nine cells, and the figures before and after
are recorded.

- [x] **Before**, on the unchanged tree: run the two guards in release with their figures printed, on
  Windows and in WSL. This run is the stage's requirement, because it is the measurement.
  `cargo test --release -p nvs-abi-probe --test perf_guards -- --nocapture an_allocation_round_trip the_memory_mark`
  and `bun nv linux-test --release -p nvs-abi-probe --test perf_guards -- --nocapture an_allocation_round_trip the_memory_mark`.
  Record each figure line in the handoff.
- [x] The counter struct and its `thread_local!`, with the 64-byte `const` assertion, replace the
  cells. `crates/nvs-runtime/src/budget.rs:265`, `crates/nvs-runtime/src/budget/gauge.rs:125`
- [x] `add`, `record` and `carry` take the struct once, and `gauge::far_from` and `gauge::report` read
  the marks from it. `crates/nvs-runtime/src/budget.rs:1437`, `crates/nvs-runtime/src/budget/gauge.rs:157`
- [x] Every other reader and writer of the moved cells, the public readers and the tests among them,
  goes through the struct. `crates/nvs-runtime/src/budget.rs:1020`
- [x] The cost sentences say one lookup: `budget`'s § *What it spends*, `gauge`'s module doc, and
  each rule fragment that counts thread-local cells or reads on this path.
  `crates/nvs-runtime/src/budget.rs:191`, `docs/rules/http-server/admission-is-arithmetic-not-a-number.md:13`,
  `docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md:55`,
  `docs/rules/observability/a-memory-peak-is-recorded-not-asked-for.md:32`
- [x] **After**: the same two runs as before. Record each figure line in the handoff and in the
  commit message, beside the before figures.

### 3. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 2 is done. One `#[repr(C)]` struct, `Counters`, in one `const` `thread_local!` (`COUNTERS`)
replaces `LIVE`, `PEAK`, `CEILING`, `REQUESTS`, `TOTAL`, `DETACHING`, `DETACHED`, `LIVE_MARK` and
`DETACHED_MARK` (`crates/nvs-runtime/src/budget.rs:265`). A `const` assertion holds the hot fields
inside 64 bytes (`HOT_SPAN`, 49 bytes). `add` and `carry` look it up once and pass it to `record` and
`shift`. `gauge::far_from` takes the mark's value, and `gauge::report` sets the marks in the struct.
`budget`'s § *What it spends*, `gauge`'s module doc and three rule fragments say one lookup, and the
chapters are rendered. The testing chapter's stage-1 sentence was unrendered and is now committed
(`7cf28ce35`). `cargo build` and `cargo clippy --all-targets -- -D warnings` are green.

The before and after figures are in the stage commit's message. Windows: update 1.62 to 1.36 ns,
pooled 4.47 to 3.88 ns. WSL's update read 1.30 and 2.34 ns on the old tree and 3.10 and 2.94 ns on
the new one, but `budget::carry`'s disassembly is the same instructions in both, so Linux code did
not change and that spread is noise. A new standing decision says why the struct is not `align(64)`.
The sweep's reds are stale: nightly `41187571578` predates the fixes, and `env-secret-proofs` has no
definition.

No test is expected to go red. The cells' values, order and every public reader's result are
unchanged; only where they are stored moved.

### Next

Stage 3, the gate: start `looper verify` with `run_in_background`, fix every failure in one pass, and
run it once more. `nvs-runtime`'s budget tests (`crates/nvs-runtime/src/budget.rs:1850`), the
count-based guards in `benches/abi-probe/tests/perf_guards.rs` and Miri's `nvs-runtime` units are
the ones this change reaches. Then the goal-end gates: `bun nv verify --doc`,
`bun nv owners --closes allocation-counters`, `bun nv ci --check`.

### Backlog

- If a steady CI run still warns on either allocator guard after this goal's push, decide the soft
  bound from the times the warning prints (`rule:testing/perf-two-mechanisms`). On WSL the update
  guard warns on two of four steady runs of either tree: its platform heap is 5.8 ns, so a quarter of
  it is 1.5 ns, which the update reaches on noise alone.
