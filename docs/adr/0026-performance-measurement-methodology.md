# ADR 0026 — Performance history is tracked by callgrind instruction counts; wall-clock stays for CI regression guards

- **Status:** Accepted
- **Date:** 2026-08-21
- **Amended by:** 0079, 0100, 0143
- **Scope:** how Novis's *own implementation* is measured and compared over time and across contributor
  machines/OSes — a historical performance dashboard, distinct from the existing per-PR regression guards in
  `benches/abi-probe/tests/perf_guards.rs` (unchanged by this ADR) and from
  [ADR 0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)'s in-language profiler
  exposed to *Novis programs* (also unchanged; that ADR is about profiling code written in Novis, this one is
  about profiling the compiler/runtime itself). **Benchmarking an Novis program's own code** is likewise not
  this ADR's: it is [ADR 0079](0079-testing-is-a-language-feature.md) § 15, which reports ADR 0018's
  deterministic counters rather than callgrind, because callgrind has no native Windows build and cannot
  resolve symbols inside JIT frames. The two remain distinct measurements of distinct things — a counter
  falls between Novis releases as the optimiser improves, which is the very trend this ADR exists to track.
- **Validated by:** `benches/abi-probe/examples/callgrind_spike.rs`, run under `valgrind --tool=callgrind`
  3.22.0 in WSL (Ubuntu on the Windows host also used for `x86_64-pc-windows-msvc` CI), against a
  Cranelift 0.128.4 JIT-compiled 8-frame call chain (the same trampoline machinery
  `perf_guards.rs` already exercises), 10,000 iterations. Three separate runs on 2026-08-21 produced the
  bit-identical instruction count `5,417,505 Ir` — see *Investigation*.

> **In short:** two mechanisms, kept deliberately separate because they answer different questions. The
> existing per-PR CI regression guards in `perf_guards.rs` are unchanged: self-relative wall-clock
> ratios/slopes (deep chain vs shallow chain, throw vs return, process vs task), already proven cross-platform-
> safe by the comments already in that file, every platform, and on any push that touches a crate whose
> cost they measure ([0143](0143-a-push-runs-the-lane-its-diff-needs-the-nightly.md) § 2), no new tooling. On top
> of that, a **historical dashboard** now exists for the separate goal of comparing Novis's performance
> across releases *and across whoever's machine happened to build it*: on every merge to `main`, a dedicated,
> non-shared Linux/WSL runner compiles a fixed benchmark workload and runs it under
> `valgrind --tool=callgrind`, and the **aggregate retired-instruction count** — not wall-clock time — is
> the number appended to an in-repo history file, because it is the one measurement that is bit-for-bit
> reproducible regardless of which machine, OS or CPU generation produced it. Wall-clock time and (once
> M3+ produces a runnable comparison) the same-run ratio against the pinned PHP 8.5.8 oracle are recorded
> alongside it per entry, as supplementary, *not* cross-machine-comparable figures. Windows and macOS dev
> machines and CI legs never run the historical leg directly — Valgrind has no native Windows build — they
> keep using the existing wall-clock guards, the same split AGENTS.md's fuzzing section already draws for
> the identical reason.

## Context

- Needed: an exact, deterministic performance measurement comparable across contributor machines/OSes and
  over time — something the existing per-PR wall-clock ratio guards in `perf_guards.rs` don't attempt, since
  a self-relative ratio only answers "did this commit regress against itself," not "is Novis getting faster in
  an absolute sense."
- Three candidates were weighed — simulated-instruction counting (Valgrind/callgrind), hardware
  instructions-retired (`perf`/ETW/Instruments), and a fixed external reference workload measured by
  wall-clock — see *Alternatives rejected* for why callgrind won.

## Investigation

- Spiked before being written down as policy, per [README.md](README.md)'s "architecture assumptions are
  tested, not remembered" rule (the same discipline ADR 0002's unwind-table premise followed) — nothing in
  the stack had run under Valgrind before.
- `benches/abi-probe/examples/callgrind_spike.rs` (an 8-frame Cranelift-JIT call chain, 10,000 iterations)
  run three times under `valgrind --tool=callgrind` in WSL produced the bit-identical count `5417505` each
  time — confirms callgrind correctly emulates Cranelift's runtime-mapped pages and that the count is
  genuinely deterministic, not merely close the way wall-clock would be.
- Limitation found: `callgrind_annotate` can't resolve symbols *inside* JIT frames (Cranelift registers no
  debug info) — they show as anonymous addresses, while ahead-of-time Rust code resolves normally.
  `PROGRAM TOTALS` (the number actually recorded) is unaffected — it's a raw retired-instruction count,
  symbolized or not; per-function drill-down inside JIT code would need a symbol-registration shim, out of
  scope since the dashboard needs an aggregate trend, not a call graph.
- Confirmed, not assumed: Valgrind has no native Windows build — needs the same WSL leg
  [AGENTS.md](../../AGENTS.md) already documents for `cargo-fuzz`, plus one new package (`valgrind`) — see
  *Decision* § 5.

## Decision

### 1. Two mechanisms, kept separate

- **CI regression guards** (`benches/abi-probe/tests/perf_guards.rs`, and its future equivalents as
  milestones add runnable Novis code): **unchanged**. Self-relative wall-clock ratios/slopes, every CI
  platform, no new tooling, loose order-of-magnitude thresholds as already documented in that file's own
  header comment. They run on a push that touched `nvs-runtime`, `nvs-codegen`, `nvs-stdlib`, `nvs-host`
  or the probes themselves, and unconditionally in the nightly and release lanes
  ([0143](0143-a-push-runs-the-lane-its-diff-needs-the-nightly.md) § 2) — a baseline cannot move without
  one of those moving, and the run is a second compile of the workspace in release profile. This answers "did this commit regress," which needs no cross-machine
  comparability because each comparison happens on one run, one machine.
- **Historical performance dashboard** (new, this ADR): answers "is Novis's own implementation getting
  faster or slower, in a sense comparable across whichever machine and OS produced each data point."

### 2. The dashboard's headline metric is the aggregate callgrind instruction count

On every merge to `main`, a **dedicated, non-shared** Linux/WSL runner (not an arbitrary shared CI agent —
noise from a co-scheduled neighbor is exactly what would defeat the determinism this ADR spikes for)
compiles the fixed benchmark workload for that commit and runs it under `valgrind --tool=callgrind`. The
`PROGRAM TOTALS` instruction count (`Ir`) is the number recorded as the historical trend line. It is chosen
over wall-clock as the headline specifically because *Investigation* now shows it is bit-for-bit
reproducible regardless of CPU generation, thermal state, or which OS's scheduler is running it — the
property a cross-machine dashboard needs and a timer cannot give.

### 3. Wall-clock and the PHP-oracle ratio are recorded alongside it, as secondary figures

Each history entry also carries that same run's wall-clock time (useful to the runner operator, not
cross-machine-comparable) and, once M3+ produces a runnable equivalent Novis program, the same-host,
same-run ratio against the pinned PHP 8.5.8 oracle already kept as a comparison baseline (per the plan's
toolchain note). The PHP ratio is not a separate reference workload invented for this ADR — it reuses the
oracle the project already keeps for correctness, on the reasoning that both runtimes experience identical
hardware and OS conditions during the same measurement, which normalizes out machine differences at least
as well as an unrelated synthetic baseline would, with no new moving part.

### 4. Storage: an in-repo, append-only file, no new service

Results are appended to `docs/perf/history.ndjson`, one JSON object per run — committed to the repository,
diffable, requiring no external dashboard service or database:

```json
{"commit":"597e830","date":"2026-08-21","workload":"abi_call_chain_depth8","instructions":5417505,"wall_clock_ns":812.3,"php_ratio":null}
```

A small script or a criterion-report-style renderer turns this into a trend chart on demand; nothing about
this ADR requires that renderer to exist before the data collection does. `php_ratio` is `null` until M3+
gives it a value — see § 3.

**The file exists as of `9ae7aa8`**, and its first entry is the workload the packed list representation is
measured by: `benches/userland/08-array-list-build.nvs`, a million appends followed by a `foreach` walk.
The three commands that produced it, run from WSL, are the whole recipe for the next one:

```
CARGO_TARGET_DIR=/tmp/nvs-linux cargo build --release -p nvs-cli
valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
    /tmp/nvs-linux/release/nvs run benches/userland/08-array-list-build.nvs      # the `I refs` line
python3 tools/bench.py 00-baseline 08-array --nvs /tmp/nvs-linux/release/nvs     # wall clock, php/nvs
```

That first entry also narrows § 2's determinism claim, which is measured of `callgrind_spike` — a bare
example binary — and **not** of a whole `nvs run`. Two consecutive runs of the workload above counted
217,254,092 and 217,254,355 instructions: reproducible to six significant figures rather than exactly,
because the process reads a file and JIT-compiles it before any of the workload runs. The number recorded
is the lower of the two. A trend line reading at that resolution is unaffected; a guard asserting equality
between two runs would not be, so no such guard exists.

### 5. `valgrind` joins the WSL one-time setup

[docs/setup.md](../setup.md) documents the one-time setup a WSL distro already needs for `cargo-fuzz`.
`valgrind` is added to that same install list — any contributor who
wants to run or verify the historical-dashboard leg locally needs it, the identical shape of dependency
`cargo-fuzz` already introduced for the identical reason (a tool with no native Windows build).

### 6. The exact workload roster is deferred, not fixed here

What benchmarks actually populate `history.ndjson` starts with `benches/abi-probe`'s existing unguarded
benches (`abi.rs`, `coroutine.rs`, `isolation.rs`, `wasm_boundary.rs`, adapted into callgrind-measurable
harnesses the way `callgrind_spike.rs` spikes one) and grows as M3 onward adds real compiled Novis programs to
measure. This is a benchmark-design question, not a language decision, and is deferred the same way other
ADRs already defer a `Core` class's exact method roster to whichever milestone builds it.

§ 3's PHP-oracle ratio now has its runnable half: `benches/userland/` holds twenty-odd pieces of ordinary
web-and-CLI code written once per engine — Novis, PHP and, per
[0100](0100-against-python-nvs-claims-the-tool-that-gets-handed-over.md) § 5, Python — and `python
tools/bench.py` runs whichever engines a case has twins for and prints the same-host ratios.
[Its README](../../benches/userland/README.md) owns what a case is; nothing about it
changes this ADR's headline metric, which stays the instruction count, for the reason § 2 gives.

## Consequences

**Positive**

- A historical trend line that means the same thing regardless of who ran it or on what hardware —
  validated by an actual spike, not assumed, per this project's own standing rule.
- Costs nothing on the existing regression-guard path: `perf_guards.rs` is untouched, still runs everywhere,
  still needs no Linux-only tooling.
- No new external service, no new long-term dependency beyond one already-precedented WSL tool install.
- Reuses the PHP 8.5.8 oracle the project keeps anyway, rather than inventing a second baseline program.

**Negative**

- **Linux/WSL becomes required for anyone verifying the historical leg locally**, on top of the existing
  `cargo-fuzz` requirement — the same trade-off already accepted for fuzzing, now paying for a second tool.
- **Per-function attribution inside JIT-compiled code is unavailable** through this mechanism; a regression
  is visible in the aggregate trend, but root-causing *where* inside the compiled program still falls back
  to wall-clock or platform-native profiling on whichever machine reproduces it.
- **A dedicated, non-shared runner is one more piece of infrastructure to keep alive** — a real ongoing cost
  this ADR does not eliminate, only accepts as necessary (a shared, noisy runner would defeat the whole
  point of choosing a deterministic metric).
- Instruction count is a proxy, not the thing Novis actually optimizes for — it can, in principle, improve
  while real latency worsens (a change trading fewer, costlier instructions for more, cheaper ones on real
  hardware — e.g. more but better-pipelined ops, or fewer cache misses at the cost of more branches). The
  wall-clock and PHP-ratio figures recorded alongside it exist specifically to catch that divergence; the
  dashboard should always be read together with them, not instruction count alone.

## Alternatives rejected

- **VM opcode/dispatch-loop counters.** Inapplicable — no interpreter tier exists.
- **Hardware PMU instructions-retired (`perf`/ETW/Instruments).** Architecture-locked and a different tool
  per OS; delivers "cross-machine, same architecture," not the cross-OS comparability needed. Callgrind's
  emulated instruction stream sidesteps both.
- **A fixed external reference workload (SHA-256/matmul loop), wall-clock ratio on every machine.**
  `perf_guards.rs`'s existing same-run ratio/slope approach already normalizes more simply for regression
  guards; callgrind's aggregate count is strictly more precise for the historical-trend purpose, with no
  extra reference binary to maintain.
- **Callgrind as the CI gate on every PR, every platform.** Valgrind's emulation overhead (one to two orders
  of magnitude) would slow every PR's CI, and Linux-only availability would skip two of three CI platforms —
  confined to a merge-to-`main`, dedicated-runner leg instead.

## Revisiting

- **The exact `history.ndjson` schema and workload roster** — open, and expected to grow through M3 onward
  as real compiled programs exist to measure; not fixed by this ADR beyond the illustrative shape in § 4.
- **Whether the dedicated runner is self-hosted hardware or a pinned cloud instance** is an infrastructure
  choice for whoever sets it up, not a language-design question this ADR resolves.
- **If Cranelift ever exposes a way to register JIT frame symbols** that callgrind (or a successor tool) can
  read, the per-function attribution gap in *Investigation* should be revisited — it would upgrade the
  dashboard from an aggregate trend to a real call-graph diff.
- **If instruction count and wall-clock/PHP-ratio are ever seen to diverge in practice** (the proxy risk
  named in *Consequences*), that is a signal to weight the secondary figures more heavily for whatever
  change caused it — not a reason to distrust the aggregate count's own determinism, which *Investigation*
  established directly.

Verification, in the order it becomes possible:

- **Now**: `benches/abi-probe/examples/callgrind_spike.rs` under `valgrind --tool=callgrind` — done, see
  *Investigation*, and rerunnable by any contributor with the WSL setup in § 5.
- ~~**M3 (Hello World)**: the first real Novis-compiled program exists; `history.ndjson` gets its first
  non-synthetic workload, and `php_ratio` gets its first real value against the pinned PHP oracle.~~
  **Done** — § 4 holds the entry and the recipe. The oracle it ran against is the WSL leg's PHP **8.5.9**,
  which is the interpreter on the machine that can run callgrind at all; the ratio is a same-host,
  same-run figure either way, which is the only property § 3 asks of it.
- **M12 (optimising JIT tier)**: the plan's existing verify line — "macro benchmarks show a multiple over
  the baseline tier and over PHP 8.5 with JIT" — is satisfied using this ADR's methodology: the same-host,
  same-run PHP-oracle ratio from § 3, not a cross-machine wall-clock claim.
