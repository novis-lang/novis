`#[Bench]` reports counted semantic work — statements executed, calls made, allocations, bytes
attributed, GC cycles — read off the same probe sites coverage and tracing use, in a counting mode
beside their timing one (`rule:testing/debug-probes`). Those numbers are **bit-identical across
machines, operating systems and architectures**, because they count what the program did rather than
what a CPU did. `bytes` costs no new instrument: memory is already attributable to a request under
an enforceable cap, and this reads that accounting.

CI may gate on the exact rows. It may not gate on wall-clock, which is printed beside them and
labelled advisory on every run.

The counters are comparable across machines and **not** across releases: an optimising tier
eliminates work, so one program's statement count falls between them. "Did my algorithm improve" is
this rule's question; "did Novis get faster" is `rule:testing/perf-two-mechanisms`'s, and the two do
not overlap. There is no per-member cost table and there will not be one — a hand-written claim
about what a `Core` member costs is a number with no guard test.

**Not on disk.** The counting mode is built: `nvs run --count` prints statements, calls, allocations,
bytes and GC cycles at exit, `gc=` last on the line (`DebugFlags::COUNT` in
`crates/nvs-runtime/src/ctx/mod.rs`, the line in `crates/nvs-cli/src/main.rs`). A GC cycle is one run of
the in-flight collector, counted in `Ctx::collect_if_asked`. `#[Bench]` is not a recognised attribute
(`crates/nvs-types/src/derive.rs:73`), no test report carries its rows
(`crates/nvs-cli/src/runner.rs:2647`), and there is no advisory wall-clock beside the counts.

**Declined:**
- **Wall-clock benchmarks with statistical treatment.** It is what other languages ship, and it
  produces a number CI cannot gate on. The counters cost less and produce one it can.
- **Instruction counts under callgrind.** They are Linux-only, many times slower than a plain run
  and blind to symbols inside JIT frames, so a user benchmarking on Windows has none
  (`rule:testing/perf-two-mechanisms` keeps them for measuring Novis itself).
- **A declared per-member cost table.** It would be more faithful for code that spends its time in
  native members, at the price of hundreds of hand-written numbers, none of them guarded.
