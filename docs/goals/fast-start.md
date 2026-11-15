---
looper: 1
title: a small program starts without paying for what it does not use
summary: "Reached when the fixed cost of one start is attributed by a recorded measurement, every part above its floor is removed or declined with its number, and a test case writes no compile cache it never reads."
context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/cache.rs, crates/nvs-types/src/core_lib.rs, crates/nvs-config/src/resolve.rs, docs/rules/programs/memory-priority.md, docs/rules/testing/perf-two-mechanisms.md]
stages:
  - id: 1
    title: one start is measured by command and counted by phase
    status: open
    context: [crates/nvs-cli/src/main.rs, tools/nv/main.ts, docs/rules/testing/perf-two-mechanisms.md]
  - id: 2
    title: a test case and a first run write no cache they wait for
    status: open
    context: [crates/nvs-test/src/run.rs, crates/nvs-cli/src/cache.rs, tools/nv/accept/runner.ts, docs/rules/packaging.md]
  - id: 3
    title: the configuration and the built-in sources are read once and parsed once
    status: open
    context: [crates/nvs-config/src/data.rs, crates/nvs-config/src/resolve.rs, crates/nvs-cli/src/extensions.rs, crates/nvs-types/src/ext_lib.rs, crates/nvs-hir/src/requires.rs, crates/nvs-cli/src/config.rs]
  - id: 4
    title: "a program pays for the `Core` signatures and symbols it names"
    status: open
    context: [crates/nvs-types/src/core_lib.rs, crates/nvs-types/src/signatures.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/cache.rs]
  - id: 5
    title: a helper thread starts when its work does
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/script.rs, crates/nvs-host/src/slice.rs]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: start-counted
    stage: 1
    name: "a small program's start does the counted work its bound allows, and no more"
    run: [cargo, test, --test, start_cost]
    timeout: 1800
    tests:
      - a_small_program_starts_within_its_counted_work
  - id: case-writes-no-cache
    stage: 2
    name: "a conformance case leaves no compile artifact behind"
    run: [cargo, test, --lib, --, a_case_run_writes_no_compile_artifact]
    timeout: 1800
    tests:
      - a_case_run_writes_no_compile_artifact
---

# a small program starts without paying for what it does not use

## Why

One `nvs` process was timed on a 16-line program (`examples/arith.nvs`), release binary, 25 interleaved
runs per shape, medians:

| Shape | ms |
|---|---|
| `nvs --version` | 16 to 19 |
| `nvs ast` | 20 |
| `nvs check` | 40 |
| `nvs run`, compile cache hit | 37 to 42 |
| `nvs run`, fresh data folder | 51 |
| the same with `NOVIS_NO_FILE_CACHE=1` | 44 |

About 20 ms lies between the end of the parse and the end of the check. Code generation and the run
add 1 to 3 ms. The test pipeline starts `nvs` about ten thousand times per full run, and a user
starts it for every command, so this fixed cost is paid more often than any other in the project.

The 20 ms is not attributed. Reading the code gives these candidates, none of them timed:

- Every `Core` class signature is built at each start (`crates/nvs-types/src/core_lib.rs:53`).
- The default `nvs.toml`, 55 KB, is read twice and parsed three times
  (`crates/nvs-config/src/data.rs:569`, `crates/nvs-config/src/resolve.rs:770`, `:994`).
- The built-in components' Novis source, 184 KB, is cloned three times
  (`crates/nvs-cli/src/extensions.rs:151`, `crates/nvs-types/src/ext_lib.rs:91`,
  `crates/nvs-hir/src/requires.rs:707`).
- The process TLS client is built before any socket exists (`crates/nvs-cli/src/config.rs:790`).
- Three helper threads are spawned and joined (`crates/nvs-cli/src/script.rs:1691`,
  `crates/nvs-host/src/slice.rs:177`, `crates/nvs-cli/src/main.rs:2648`).
- About 1,000 helper symbols are put into a new map per loaded unit and per compile
  (`crates/nvs-cli/src/cache.rs:1717`, `crates/nvs-codegen/src/lib.rs:2441`).

A test case pays more. Each case runs with a data folder of its own
(`crates/nvs-test/src/run.rs:520`), so it compiles twice, writes an artifact with an `fsync`
(`crates/nvs-cli/src/cache.rs:1568`) and then deletes it. That is the measured 7 ms of 51.

## Standing decisions

**Limits: what stops this goal.** Each line is a rule a session follows without asking.

- **Counts decide, the clock never does.** Two runs of the same shape differed by 4 ms here, so a
  saving under 5 ms cannot be seen on the clock. Stage 1 records, per start phase, the allocations,
  the files read, the bytes parsed and the threads spawned. A candidate is built only when stage 1's
  table gives its phase at least 5% of the start's allocations, or one file read of 16 KiB or more,
  or one thread.
- **A candidate under its floor is not built.** Its item is ticked with the number, and the number
  goes into the `# What it costs` of the module it lives in.
- **An item gets one attempt.** If its build or its named test is not green when its session ends,
  its uncommitted edits are discarded, the item is ticked as declined, and one line in `### Backlog`
  says what blocked it.
- **A stage gets two sessions.** `### State` holds one line per stage with the sessions used. At the
  ceiling, every open item of the stage moves to `### Backlog` as declined and the next stage starts.
- **No item is added.** A new finding goes to `### Backlog`. It never joins a stage.
- **No target figure.** The goal is reached when every listed candidate is built or declined. It is
  not held open for a number of milliseconds.

**The measuring command is `bun nv start-cost`.** It runs `--version`, `ast`, `check` and `run` on a
shared and a fresh data folder, 25 interleaved runs each, and prints medians beside the counts. A
manual step done twice becomes a command. **What it spends:** about a minute per run, one heavy slot.

**The phase counts are debug-only.** `rule:testing/perf-two-mechanisms` allows no counter on a
production path for a test. The counts use the thread-local counters `nvs check --count` already
prints. Safe default if that output cannot name a phase: the phase is counted by a test that calls
the phase's function directly.

**The watchdog thread stays as it is.** It enforces the CPU limit, and an enforcement thread is not
traded for a thread spawn. Declined: starting it on first enrolment.

**`Core` signatures become lazy per class, not generated.** A class is lowered on its first `get`,
over an interner that first receives every distinct `Core` type shape, so type ids stay stable.
Declined: generating the table at build time, which is a second home for the registry. Safe default
if the frozen base interner cannot take lazy entries without changing an id: decline the item.
**What it spends:** one `OnceLock` per `Core` class.

**The cold run still compiles for the JIT and for the artifact, on two threads.**
`crates/nvs-cli/src/cache.rs:1657` chose two walks so a run never waits for the loader. The object
walk moves to a scoped thread that is joined before the unit is returned. Safe default if
`nvs_ir::Program` is not `Sync`: decline the item. **What it spends:** one thread for the length of
one compile.

**The artifact is no longer synced.** A torn file fails its checksum and is a miss
(`rule:packaging/a-bad-cache-entry-is-a-miss-never-an-error`). The rename stays
(`rule:packaging/a-writer-publishes-by-one-atomic-rename-and-never-a-lock`).

**Lanes:** one lane, in order. Stages 3 to 5 all edit `crates/nvs-cli/src/main.rs`.

## Stages

### 1. one start is measured by command and counted by phase

**Does:** add `bun nv start-cost`, record the per-phase counts of one start, and decide stages 3 to 5
by the table.

**Split:** an `Explore` subagent lists what `nvs check --count` prints and where each counter is
bumped, as `file.rs:NN`, before the first edit.

- [ ] `tools/nv/cmd/start-cost.ts:1`: the command, registered in `tools/nv/main.ts:24`, with a Bun
  test of its argument handling (`rule:testing/perf-two-mechanisms`).
- [ ] `crates/nvs-cli/src/main.rs:1174`: the start phases are named to the counters: configuration,
  built-in sources, grants, `Core` signatures, collection, check
  (`rule:testing/perf-two-mechanisms`).
- [ ] `crates/nvs-cli/tests/start_cost.rs:1`: `a_small_program_starts_within_its_counted_work` pins
  each phase's counts with the bound stage 1 measured (`rule:testing/perf-two-mechanisms`).
- [ ] `docs/goals/fast-start.md:1`: write the table into `## Standing decisions` as **The measured
  start**, and tick every candidate of stages 3 to 5 that is under its floor
  (`rule:programs/memory-priority`).

### 2. a test case and a first run write no cache they wait for

**Does:** stop a test case from compiling for a cache it deletes, and stop a first run from waiting
on the disk.

- [ ] `crates/nvs-test/src/run.rs:588`: a case's `nvs` runs with `NOVIS_NO_FILE_CACHE=1`, and a case
  that tests the compile cache opts back in with a directive
  (`rule:packaging/a-bad-cache-entry-is-a-miss-never-an-error`).
- [ ] `tools/nv/accept/runner.ts:138`: the Bun harness sets the same variable for every proof run
  (`rule:testing/feature-proofs`).
- [ ] `crates/nvs-test/src/run.rs:520`: `a_case_run_writes_no_compile_artifact`
  (`rule:testing/perf-two-mechanisms`).
- [ ] `crates/nvs-cli/src/cache.rs:1568`: `write_temp` writes and closes without `sync_all`, and
  `docs/rules/packaging.md:253` says what a crash costs
  (`rule:packaging/a-writer-publishes-by-one-atomic-rename-and-never-a-lock`).
- [ ] `crates/nvs-cli/src/cache.rs:1687`: the object compile runs on a scoped thread beside the JIT
  compile, and the doc comment at `:1657` says so (`rule:programs/memory-priority`).

### 3. the configuration and the built-in sources are read once and parsed once

**Does:** remove the repeated reads, parses and copies of data that is the same at every start.

**Gate:** each item is built only when stage 1's table puts its phase at or over its floor.

- [ ] `crates/nvs-config/src/data.rs:569`: the default file's hash is a compile-time constant, and the
  file is read once per start (`rule:config/the-config-is-an-immutable-snapshot`).
- [ ] `crates/nvs-config/src/resolve.rs:770`: one parse into a table, and the typed `Config` is
  derived from it (`rule:config/the-config-is-an-immutable-snapshot`).
- [ ] `crates/nvs-cli/src/extensions.rs:151`: `Manifest.source` and `ExtensionFile.text` are shared,
  not cloned, through `crates/nvs-types/src/ext_lib.rs:91` and `crates/nvs-hir/src/requires.rs:707`
  (`rule:programs/memory-priority`).
- [ ] `crates/nvs-cli/src/config.rs:790`: with no `[http.client.tls]` table, the TLS client is built
  by its first handshake (`rule:programs/memory-priority`).

### 4. a program pays for the `Core` signatures and symbols it names

**Does:** build a `Core` class's signatures on first use, and build the helper symbol table once per
process.

**Gate:** as stage 3.

- [ ] `crates/nvs-types/src/core_lib.rs:53`: `base()` seeds a class on its first `get`
  (`rule:programs/memory-priority`).
- [ ] `crates/nvs-cli/src/cache.rs:1717`: `this_process` reads one process-wide table, shared with
  `crates/nvs-codegen/src/lib.rs:2441` (`rule:programs/memory-priority`).

### 5. a helper thread starts when its work does

**Does:** start the revalidation thread and the slice clock on first need.

**Gate:** as stage 3.

- [ ] `crates/nvs-cli/src/script.rs:1691`: `nvs-revalidate` starts on the first
  `Compiler::compiled` call (`rule:programs/memory-priority`).
- [ ] `crates/nvs-host/src/slice.rs:177`: the slice clock starts when the scheduler gets its second
  task (`rule:concurrency/the-parking-contract`).

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.
- [ ] `crates/nvs-cli/src/main.rs:730`: run `bun nv start-cost` once and write its table into the
  module doc's `# What it costs`.

## Handoff

### State

Nothing is built. Sessions used: stage 1: 0, stage 2: 0, stage 3: 0, stage 4: 0, stage 5: 0.

### Next

Stage 1, in one session. The files are `tools/nv/cmd/start-cost.ts`, `tools/nv/main.ts` and
`crates/nvs-cli/src/main.rs`.

- [ ] `tools/nv/cmd/start-cost.ts:1`: the command.
- [ ] `crates/nvs-cli/src/main.rs:1174`: the phases named to the counters.
- [ ] `crates/nvs-cli/tests/start_cost.rs:1`: the count guard.

### Backlog

- Declined before the run: a fast path that skips the full argument parser for `nvs run`. Two
  parsers drift, and the saving is not measured.
- Declined before the run: creating the reactor on first park
  (`rule:concurrency/the-parking-contract`).
