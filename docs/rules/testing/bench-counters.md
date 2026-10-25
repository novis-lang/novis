`#[Bench]` reports counted semantic work — statements executed, calls made, allocations, bytes
attributed, GC cycles — read off the same probe sites coverage and tracing use, in a counting mode
beside their timing one (`rule:testing/debug-probes`). Those numbers are **bit-identical across
machines, operating systems and architectures**, because they count what the program did rather than
what a CPU did. `bytes` costs no new instrument: memory is already attributable to a request under
an enforceable cap, and this reads that accounting.

CI may gate on the exact rows. It may not gate on wall-clock, which is printed beside them and
labelled advisory on every run.

**A bench is test code that runs only under `nvs test --bench`.** `#[Bench]` (`Core\Test\Bench`)
marks a method of the shape `#[Test]` gives one, in a class `nvs test` discovers, so a bench takes
fixtures, the constructor as setup, hooks and inheritance as a test does, and a build leaves it out
(`rule:testing/tests-never-reach-a-build`). It takes `skip` and `because`, at the types
`#[Test]`'s roster gives them, and no data row. A plain `nvs test` runs no bench and does not count
one as left out; `--bench` runs every bench and no test, one at a time, each once. A bench passes
with no assertion, because what it claims is its rows, and an assertion that fails still fails it.
Why once: its counts are the same on every run, so a repeat or iteration option buys nothing
(simplicity).

The rows are the bench's call alone, counted on its own isolate: the thread's empty array is made
before the window opens, so a bench's rows do not depend on which benches ran before it. A task
the bench spawns counts on a context of its own, which the rows do not include, as `nvs run
--count`'s do not. The text report prints them as `nvs run --count` does, under the bench's line,
and the JSON report gives each bench `counts` and `advisoryWallMs` (`rule:testing/report-formats`).

The counters are comparable across machines and **not** across releases: an optimising tier
eliminates work, so one program's statement count falls between them. "Did my algorithm improve" is
this rule's question; "did Novis get faster" is `rule:testing/perf-two-mechanisms`'s, and the two do
not overlap. There is no per-member cost table and there will not be one — a hand-written claim
about what a `Core` member costs is a number with no guard test.

**Not on disk.** The counting mode is built: `nvs run --count` prints statements, calls, allocations,
bytes and GC cycles at exit, `gc=` last on the line (`DebugFlags::COUNT` in
`crates/nvs-runtime/src/ctx/mod.rs`, the line in `crates/nvs-cli/src/main.rs`). A GC cycle is one run of
the in-flight collector, counted in `Ctx::collect_if_asked`. `#[Bench]` and `nvs test --bench` are
built (`crates/nvs-types/src/testing.rs`'s `BENCH_OPTIONS`, `crates/nvs-cli/src/runner.rs`'s
`Rows`). The feature's proofs are not: no `about.md`, examples, Novis test, attack or bench under
`docs/examples/lang/testing/bench-and-nvs-test-bench/`.

**Declined:**
- **Wall-clock benchmarks with statistical treatment.** It is what other languages ship, and it
  produces a number CI cannot gate on. The counters cost less and produce one it can.
- **Instruction counts under callgrind.** They are Linux-only, many times slower than a plain run
  and blind to symbols inside JIT frames, so a user benchmarking on Windows has none
  (`rule:testing/perf-two-mechanisms` keeps them for measuring Novis itself).
- **A declared per-member cost table.** It would be more faithful for code that spends its time in
  native members, at the price of hundreds of hand-written numbers, none of them guarded.
