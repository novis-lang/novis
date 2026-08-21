# ADR 0026 — Performance history is tracked by callgrind instruction counts; wall-clock stays for CI regression guards

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** how MWL's *own implementation* is measured and compared over time and across contributor
  machines/OSes — a historical performance dashboard, distinct from the existing per-PR regression guards in
  `benches/abi-probe/tests/perf_guards.rs` (unchanged by this ADR) and from
  [ADR 0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)'s in-language profiler
  exposed to *MWL programs* (also unchanged; that ADR is about profiling code written in MWL, this one is
  about profiling the compiler/runtime itself).
- **Relates to:** [0002](0002-error-propagation.md) (owns the self-relative ratio/slope pattern this ADR
  builds on top of, not replaces), [0004](0004-memory-for-simplicity.md) (priority 3, latency/throughput,
  is the reason wall-clock stays authoritative for regression guards even though it is not cross-machine
  comparable), [0006](0006-isolated-script-execution.md) (`benches/abi-probe`'s process-vs-task ratio guard
  is the existing precedent for "ratio cancels machine differences"), [0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
  (the neighboring, non-overlapping concern named above), [docs/adr/README.md](README.md) § *Architecture
  assumptions are tested, not remembered* (the standing rule this ADR's own spike follows), CLAUDE.md's
  WSL-for-`cargo-fuzz` precedent (the same shape of Linux-only-tooling gap, now extended to Valgrind)
- **Validated by:** `benches/abi-probe/examples/callgrind_spike.rs`, run under `valgrind --tool=callgrind`
  3.22.0 in WSL (Ubuntu on the Windows host also used for `x86_64-pc-windows-msvc` CI), against a
  Cranelift 0.128.4 JIT-compiled 8-frame call chain (the same trampoline machinery
  `perf_guards.rs` already exercises), 10,000 iterations. Three separate runs on 2026-08-21 produced the
  bit-identical instruction count `5,417,505 Ir` — see *Investigation*.

> **In short:** two mechanisms, kept deliberately separate because they answer different questions. The
> existing per-PR CI regression guards in `perf_guards.rs` are unchanged: self-relative wall-clock
> ratios/slopes (deep chain vs shallow chain, throw vs return, process vs task), already proven cross-platform-
> safe by the comments already in that file, running on every push, every platform, no new tooling. On top
> of that, a **historical dashboard** now exists for the separate goal of comparing MWL's performance
> across releases *and across whoever's machine happened to build it*: on every merge to `main`, a dedicated,
> non-shared Linux/WSL runner compiles a fixed benchmark workload and runs it under
> `valgrind --tool=callgrind`, and the **aggregate retired-instruction count** — not wall-clock time — is
> the number appended to an in-repo history file, because it is the one measurement that is bit-for-bit
> reproducible regardless of which machine, OS or CPU generation produced it. Wall-clock time and (once
> M3+ produces a runnable comparison) the same-run ratio against the pinned PHP 8.5.8 oracle are recorded
> alongside it per entry, as supplementary, *not* cross-machine-comparable figures. Windows and macOS dev
> machines and CI legs never run the historical leg directly — Valgrind has no native Windows build — they
> keep using the existing wall-clock guards, the same split CLAUDE.md's fuzzing section already draws for
> the identical reason.

## Context

A chatbot brainstorm suggested three options for exact, deterministic, cross-machine/OS performance
comparison: (1) simulated-instruction counting — either an interpreter's own opcode-dispatch counter, or an
external emulator like Valgrind/callgrind; (2) hardware "instructions retired" via the CPU's PMU
(`perf stat -e instructions:u` and equivalents); (3) a fixed external reference workload, measured
wall-clock alongside the real benchmark, to normalize out clock-speed differences. None of the three were
evaluated against what MWL has already decided:

- **Option 1's interpreter half does not apply.** "Cranelift JIT as the only execution tier, no
  interpreter" is a project-start decision ([README.md](README.md) § *Decisions taken at project start*).
  There is no opcode-dispatch loop to instrument — every MWL function becomes native code, so only the
  emulator half (Valgrind/callgrind) is a candidate at all.
- **Option 2 is architecture-locked and multi-tool.** `perf` (Linux), ETW (Windows) and Instruments (macOS)
  are three different tools with three different overhead/precision profiles, and none of them make an
  x86_64 count comparable to an arm64 one. It answers "cross-machine, same architecture," not "cross-machine
  and OS" as asked.
- **Option 3, taken literally (an unrelated SHA-256/matmul baseline), is already bettered by a pattern this
  project uses today.** `benches/abi-probe/tests/perf_guards.rs` normalizes out per-machine constant
  overhead by taking **ratios and slopes within the same run** — a 2-frame vs 18-frame call chain, a thrown
  vs a returned call, an OS process vs an in-process task — rather than comparing against an unrelated
  reference binary. The file's own comments already reason about this explicitly (`an_os_process_...`:
  "`CreateProcess` is dearer than `fork`+`exec`, so a Linux runner will report a smaller ratio; the 20x guard
  is set low enough to hold everywhere"). This is the right tool for **regression guards** and needs no
  change.

What none of the above gives is a **historical trend line that is meaningfully comparable across an
arbitrary set of contributor machines and CI runners over months of commits** — the goal actually asked
for. A wall-clock number from one laptop says nothing next to one from a cloud CI runner two years later on
different hardware; a ratio-based guard test answers "did this commit regress *relative to itself*," not
"is MWL, in an absolute and comparable sense, getting faster." That gap is what this ADR closes, without
touching the regression-guard mechanism that already works.

## Investigation

**Does callgrind even work against Cranelift's runtime-generated code?** Nothing in this project's stack has
run under Valgrind before — every existing guard test measures wall-clock directly. Per
[README.md](README.md)'s own rule ("architecture assumptions are tested, not remembered"), this was spiked
before being written down as policy, the same way ADR 0002's unwind-table premise was spiked before being
assumed.

`benches/abi-probe/examples/callgrind_spike.rs` reuses the exact `Probe::compile_chain`/`call` machinery
`perf_guards.rs` already exercises — an 8-frame Cranelift-JIT-compiled call chain, called 10,000 times — and
was run three separate times under `valgrind --tool=callgrind` in WSL:

```text
==2388== Collected : 5417505
==2403== Collected : 5417505
==2408== Collected : 5417505
```

Bit-identical across all three runs. This confirms callgrind's instruction emulation handles Cranelift's
runtime-mapped executable pages correctly (it does not simply skip or approximate unmapped-at-startup code)
and that the count is genuinely deterministic — not merely "close," the way a wall-clock figure would be
even in the best case.

**A real limitation surfaced too.** `callgrind_annotate` cannot resolve symbols *inside* JIT-compiled
frames — Cranelift registers no debug info callgrind can read, so each JIT-emitted function shows as an
anonymous `???:0x000000000403f000`-style line rather than a name, while ahead-of-time-compiled Rust code
(the `probe_double` helper) resolves normally:

```text
260,000 ( 4.80%)  benches/abi-probe/src/lib.rs:mwl_abi_probe::probe_double [...]
210,000 ( 3.88%)  ???:0x000000000403f000 [???]
200,000 ( 3.69%)  ???:0x000000000403f059 [???]
```

`PROGRAM TOTALS` — the number this ADR's dashboard actually records — is unaffected by this: it is a raw
count of every instruction retired, symbolized or not. Per-function drill-down *inside compiled MWL code*
is not available this way without a JIT symbol-registration shim, which is out of scope: a historical trend
line needs the aggregate, not a call graph, and diagnosing *where* a regression lives already has a
different tool (wall-clock sampling profilers, or `perf`/ETW/Instruments locally on whichever platform
reproduces it).

**Platform reality confirmed, not assumed.** Valgrind has no native Windows build. Building and running the
spike required the same WSL leg [CLAUDE.md](../../CLAUDE.md) already documents for `cargo-fuzz`, plus one
new one-time package (`valgrind` itself, via `apt-get`) that setup did not previously need. This is now
folded into that same setup section — see *Decision* § 5.

## Decision

### 1. Two mechanisms, kept separate

- **CI regression guards** (`benches/abi-probe/tests/perf_guards.rs`, and its future equivalents as
  milestones add runnable MWL code): **unchanged**. Self-relative wall-clock ratios/slopes, every push,
  every CI platform, no new tooling, loose order-of-magnitude thresholds as already documented in that
  file's own header comment. This answers "did this commit regress," which needs no cross-machine
  comparability because each comparison happens on one run, one machine.
- **Historical performance dashboard** (new, this ADR): answers "is MWL's own implementation getting
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
cross-machine-comparable) and, once M3+ produces a runnable equivalent MWL program, the same-host,
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

### 5. `valgrind` joins the WSL one-time setup

[CLAUDE.md](../../CLAUDE.md)'s "Fuzzing on Windows: use WSL" section documents the one-time setup already
needed for `cargo-fuzz`. `sudo apt-get install -y valgrind` is added to that same list — any contributor who
wants to run or verify the historical-dashboard leg locally needs it, the identical shape of dependency
`cargo-fuzz` already introduced for the identical reason (a tool with no native Windows build).

### 6. The exact workload roster is deferred, not fixed here

What benchmarks actually populate `history.ndjson` starts with `benches/abi-probe`'s existing unguarded
benches (`abi.rs`, `coroutine.rs`, `isolation.rs`, `wasm_boundary.rs`, adapted into callgrind-measurable
harnesses the way `callgrind_spike.rs` spikes one) and grows as M3 onward adds real compiled MWL programs to
measure. This is a benchmark-design question, not a language decision, and is deferred the same way other
ADRs already defer a `Core` class's exact method roster to whichever milestone builds it.

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
- Instruction count is a proxy, not the thing MWL actually optimizes for — it can, in principle, improve
  while real latency worsens (a change trading fewer, costlier instructions for more, cheaper ones on real
  hardware — e.g. more but better-pipelined ops, or fewer cache misses at the cost of more branches). The
  wall-clock and PHP-ratio figures recorded alongside it exist specifically to catch that divergence; the
  dashboard should always be read together with them, not instruction count alone.

## Alternatives rejected

- **VM opcode/dispatch-loop counters.** Inapplicable outright — no interpreter tier exists, by a
  project-start decision this ADR does not revisit.
- **Hardware PMU instructions-retired (`perf`/ETW/Instruments).** Rejected per this ADR's own scoping:
  architecture-locked (an x86_64 and an arm64 runner would disagree even for identical semantics), and a
  different tool with different overhead per OS, which delivers "cross-machine, same architecture," not the
  "cross-machine and OS" the goal actually asked for. Callgrind's emulated instruction stream sidesteps both
  problems at once.
- **A fixed external reference workload (SHA-256/matmul loop), wall-clock ratio on every machine.**
  Rejected: the ratio-based ideas already used in `perf_guards.rs` — differencing within the same run
  instead of against an unrelated program — already achieve normalization more simply for regression-guard
  purposes, and now that *Investigation* confirms callgrind works at all, its aggregate count is strictly
  more precise for the historical-trend purpose, with no extra reference binary to build and maintain.
- **Callgrind as the CI gate on every PR, every platform.** Rejected: Valgrind's own emulation overhead
  (commonly one to two orders of magnitude) would slow every PR's CI run substantially, and its Linux-only
  availability would silently skip two of the project's three CI platforms. Confined to a merge-to-`main`,
  dedicated-runner leg instead, which pays that cost once per merge rather than once per push.

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
- **M3 (Hello World)**: the first real MWL-compiled program exists; `history.ndjson` gets its first
  non-synthetic workload, and `php_ratio` gets its first real value against the pinned PHP 8.5.8 oracle.
- **M12 (optimising JIT tier)**: the plan's existing verify line — "macro benchmarks show a multiple over
  the baseline tier and over PHP 8.5 with JIT" — is satisfied using this ADR's methodology: the same-host,
  same-run PHP-oracle ratio from § 3, not a cross-machine wall-clock claim.
