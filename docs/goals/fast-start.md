---
looper: 1
title: a small program starts without paying for what it does not use
summary: "Reached when the fixed cost of one start is attributed by a recorded measurement, every part above its floor is removed or declined with its number, and a test case writes no compile cache it never reads."
context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/cache.rs, crates/nvs-types/src/core_lib.rs, crates/nvs-config/src/resolve.rs, docs/rules/programs/memory-priority.md, docs/rules/testing/perf-two-mechanisms.md]
stages:
  - id: 1
    title: one start is measured by command and counted by phase
    status: done
    context: [crates/nvs-cli/src/main.rs, tools/nv/main.ts, docs/rules/testing/perf-two-mechanisms.md]
  - id: 2
    title: a test case and a first run write no cache they wait for
    status: done
    context: [crates/nvs-test/src/run.rs, crates/nvs-cli/src/cache.rs, tools/nv/accept/runner.ts, docs/rules/packaging.md]
  - id: 3
    title: the configuration and the built-in sources are read once and parsed once
    status: done
    context: [crates/nvs-config/src/data.rs, crates/nvs-config/src/resolve.rs, crates/nvs-cli/src/extensions.rs, crates/nvs-types/src/ext_lib.rs, crates/nvs-hir/src/requires.rs, crates/nvs-cli/src/config.rs]
  - id: 4
    title: "a program pays for the `Core` signatures and symbols it names"
    status: done
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

**The measured start.** `bun nv start-cost` on Windows, a one-line program, the debug `nvs check
--count` (the `run --count` table is the same but for 440 configuration allocations):

| phase | allocations | bytes | settings | resolved | share |
|---|---:|---:|---:|---:|---:|
| configuration | 446 | 1,532,658 | 1 | 2 | 1.3% |
| built_in | 6,729 | 1,259,714 | 0 | 0 | 19.4% |
| grants | 25 | 1,229 | 0 | 0 | 0.1% |
| core | 16,624 | 1,739,758 | 0 | 0 | 47.9% |
| collection | 1,196 | 319,589 | 0 | 1 | 3.4% |
| check | 9,659 | 697,032 | 0 | 0 | 27.9% |

No phase reads a source file after the entry. Release medians of 25 runs, shared then fresh data
folder: `--version` 10.0 / 11.6 ms, `ast` 10.2 / 11.8, `check` 16.4 / 21.1, `run` 18.6 / 31.5. The
13 ms a fresh `run` adds is stage 2's cache write.

What the table decides, by the floors under **Limits**:

- Stage 3's configuration items are at the floor by the file, not the allocations: the shipped
  `crates/nvs-config/src/default.toml` is 78,788 bytes, parsed at every start.
- Stage 3's extension item is at the floor: `built_in` is 19.4%.
- Stage 4's `Core` item is at the floor: `core` is 47.9%, the largest phase.
- Stage 5's two items are each one thread, so each is at the floor by definition.
- Three candidates run outside the six front-end phases, so this table does not price them: the TLS
  client (stage 3, `install_tls_client` runs after the front end), the helper symbol table (stage
  4, `this_process` runs in the compile) and the threads (stage 5). Each one's session first wraps
  its site in a `phases::start()` and `end` with the nearest phase, reads the count, and builds it
  only at its floor. That mark is a measurement and is removed before the commit.
- `check` (27.9%) has no candidate in this goal. A finding about it goes to `### Backlog`.

## Stages

### 1. one start is measured by command and counted by phase

**Does:** add `bun nv start-cost`, record the per-phase counts of one start, and decide stages 3 to 5
by the table.

**Split:** an `Explore` subagent lists what `nvs check --count` prints and where each counter is
bumped, as `file.rs:NN`, before the first edit.

- [x] `tools/nv/cmd/start-cost.ts:1`: the command, registered in `tools/nv/main.ts:24`, with a Bun
  test of its argument handling (`rule:testing/perf-two-mechanisms`).
- [x] `crates/nvs-cli/src/main.rs:1174`: the start phases are named to the counters: configuration,
  built-in sources, grants, `Core` signatures, collection, check
  (`rule:testing/perf-two-mechanisms`).
- [x] `crates/nvs-cli/tests/start_cost.rs:1`: `a_small_program_starts_within_its_counted_work` pins
  each phase's counts with the bound stage 1 measured (`rule:testing/perf-two-mechanisms`).
- [x] `docs/goals/fast-start.md:1`: write the table into `## Standing decisions` as **The measured
  start**, and tick every candidate of stages 3 to 5 that is under its floor
  (`rule:programs/memory-priority`).

### 2. a test case and a first run write no cache they wait for

**Does:** stop a test case from compiling for a cache it deletes, and stop a first run from waiting
on the disk.

- [x] `crates/nvs-test/src/run.rs:588`: a case's `nvs` runs with `NOVIS_NO_FILE_CACHE=1`, and a case
  that tests the compile cache opts back in with a directive
  (`rule:packaging/a-bad-cache-entry-is-a-miss-never-an-error`). Built as `run::case_env`, and the
  directive is `--COMPILE_CACHE--` `on`.
- [x] `tools/nv/accept/runner.ts:138`: the Bun harness sets the same variable for every proof run
  (`rule:testing/feature-proofs`). Built in `tools/nv/proofs/run.ts`'s `spawnProof`, which every
  proof run goes through; a `.cmd` proof keeps its own command line's cache.
- [x] `crates/nvs-test/src/run.rs:520`: `a_case_run_writes_no_compile_artifact`
  (`rule:testing/perf-two-mechanisms`).
- [x] `crates/nvs-cli/src/cache.rs:1568`: `write_temp` writes and closes without `sync_all`, and
  `docs/rules/packaging.md:253` says what a crash costs
  (`rule:packaging/a-writer-publishes-by-one-atomic-rename-and-never-a-lock`). A file shorter than a
  header or with an all-zero header is now corrupt and deleted, so a torn file cannot hold a key cold.
- [x] `crates/nvs-cli/src/cache.rs:1687`: the object compile runs on a scoped thread beside the JIT
  compile, and the doc comment at `:1657` says so (`rule:programs/memory-priority`). Built as
  `compile_beside_object`.

### 3. the configuration and the built-in sources are read once and parsed once

**Does:** remove the repeated reads, parses and copies of data that is the same at every start.

**Gate:** each item is built only when stage 1's table puts its phase at or over its floor.

- [x] `crates/nvs-config/src/data.rs:569`: the default file's hash is a compile-time constant, and the
  file is read once per start (`rule:config/the-config-is-an-immutable-snapshot`).
  - [x] `build.rs` writes the hash, and `is_template` compares without building the template
  - [x] `config_file` keeps the text, and `Unowned::read_config` takes it once
  - [x] measured: shared-folder `run` configuration bytes 1,000,414 to 842,036
- [x] `crates/nvs-config/src/resolve.rs:770`: one parse into a table, and the typed `Config` is
  derived from it (`rule:config/the-config-is-an-immutable-snapshot`).
  - [x] `file::parse_document` keeps the spanned document
  - [x] `settings::check_file` and the merge table read that document
  - [x] measured: `run` configuration bytes 1,531,405 to 924,973
- [x] `crates/nvs-cli/src/extensions.rs:151`: `Manifest.source` and `ExtensionFile.text` are shared,
  not cloned, through `crates/nvs-types/src/ext_lib.rs:91` and `crates/nvs-hir/src/requires.rs:707`
  (`rule:programs/memory-priority`).
  - [x] both texts are `Arc<str>`, and `carried_source` copies only into the source map
  - [x] measured: `built_in` bytes 1,259,714 to 1,075,168, `collection` 318,581 to 133,472
- [x] `crates/nvs-cli/src/config.rs:790`: with no `[http.client.tls]` table, the TLS client is built
  by its first handshake (`rule:programs/memory-priority`). Declined under its floor: 18 allocations
  and 33 KB, no file read and no thread. The number is in `config.rs`'s `# What it costs`.

### 4. a program pays for the `Core` signatures and symbols it names

**Does:** build a `Core` class's signatures on first use, and build the helper symbol table once per
process.

**Gate:** as stage 3.

- [x] `crates/nvs-types/src/core_lib.rs:53`: `base()` seeds a class on its first `get`
  (`rule:programs/memory-priority`). Built: `core` went from 16,624 to 8,109 allocations.
- [x] `crates/nvs-cli/src/cache.rs:1717`: `this_process` reads one process-wide table, shared with
  `crates/nvs-codegen/src/lib.rs:2441` (`rule:programs/memory-priority`). Declined: the JIT's
  table is 1,266 allocations per compile and the warm hit's 123, under the floor of about 1,300.

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

Stages 1 to 4 are done. Stage 4 landed in two commits. A `Core` class's signatures are built on
its first lookup (`core_lib::Natives`): the base interns every `Core` type first, so a class built
later adds no type and changes no id, and `build_class` asserts that. `core` went from 16,624 to
8,109 allocations; 6,688 of what is left is the type pre-pass, whose lowering allocates even on an
interner hit. The helper symbol tables were priced and declined, and each site's comment has its
number. Sessions used: stage 1: 1, stage 2: 1, stage 3: 1, stage 4: 1, stage 5: 0.

The sweep's findings before this session needed no change. `env-secret-proofs` has no check and no
log in the tree. Nightly 43581848915 is the same run the last handoff named, older than
`8bd6b6eda9`'s Miri fix.

Measure from a directory with no `nvs.toml` of its own, such as `.agent-tmp/`: the repository's
own `nvs.toml` adds about 33,000 configuration allocations that a user's start does not pay. The
front end of a one-line `nvs check` is now about 26,000 allocations, so the 5% floor is about 1,300.

### Next

Stage 5, in one session. The files are `crates/nvs-cli/src/script.rs` and
`crates/nvs-host/src/slice.rs`. Each item is one thread, so each is at the floor.

- [ ] `crates/nvs-cli/src/script.rs:1691`: `nvs-revalidate` starts on the first
  `Compiler::compiled` call.
- [ ] `crates/nvs-host/src/slice.rs:177`: the slice clock starts when the scheduler gets its second
  task (`rule:concurrency/the-parking-contract`).

Tests that may go red at stage 6: `nvs-types`'s tests that read `SignatureTable::iter` and expect
a `Core` class in it (none is known; only `check.rs` and `mark_overridden_methods` read it), a
test that pins a `TypeId` number of a native type (the base interns `Core` before the error
classes now, as before, but in another order), the conformance cases that relied on a case writing
a compile artifact (none is known), `nvs-cli`'s
`a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names`, whose cold arm is shorter,
and a `nvs-config` test that expected an empty unknown header in the merge table before
`Merge::finish` (none is known; `finish` dropped it already).

### Backlog

- Declined before the run: a fast path that skips the full argument parser for `nvs run`. Two
  parsers drift, and the saving is not measured.
- Declined before the run: creating the reactor on first park
  (`rule:concurrency/the-parking-contract`).
- Found in stage 3: `built_in` still makes 6,699 allocations, nearly all of them the one parse of
  the built-in manifests' JSON and the copy of each `Manifest` in `with_built_in`. Not in a stage.
- Found in stage 4: the `Core` type pre-pass makes 6,688 allocations, 4,948 of them transient
  (`QName::parse` and argument vectors in `core_lib::lower`, paid again on every interner hit).
  Lowering a type without building its `Ty` first would remove most of them. Not in a stage.
