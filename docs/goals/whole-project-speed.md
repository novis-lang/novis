---
looper: 1
title: "reading a whole project costs the compiler's work and little more"
summary: "a cold editor index, nvs check of a class file and an uncached compile spend their time in the compiler, each file-system fact is learned once per run, the editor indexes on every core, and count-based tests keep it so"
context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/document.rs, crates/nvs-hir/src/lenders.rs, docs/rules/ide/the-index-is-incremental-and-persistent.md]
stages:
  - id: 1
    title: "nvs check --count and the editor index report the file-system work they did, and every whole-project path is measured"
    status: done
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/check.rs, crates/nvs-diagnostics/src/source.rs, crates/nvs-hir/src/requires.rs, crates/nvs-cli/tests/check_command.rs, docs/rules/testing/bench-counters.md, crates/nvs-lsp/src/server.rs]
  - id: 2
    title: "the editor reads each settings file once per change, and hashes each extension set once"
    status: open
    context: [crates/nvs-lsp/tests/index.rs, crates/nvs-lsp/src/session.rs]
  - id: 3
    title: "nvs check of a class file, an uncached compile, nvs test of a directory and nvs fmt of a tree learn each file-system fact once"
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/tests/check_borrow.rs, crates/nvs-cli/src/fmt.rs, docs/rules/ide/an-autoloaded-file-borrows-its-programs-map.md]
  - id: 4
    title: "an index entry is analysed by a function that owns everything it reads, and merged into the index apart from that"
    status: open
    context: [crates/nvs-lsp/src/session.rs, crates/nvs-lsp/src/parse_cache.rs, docs/rules/ide/the-server-is-synchronous.md]
  - id: 5
    title: "the editor settles its index on every core, and the index it builds is the serial one"
    status: open
    context: [crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/progress.rs, docs/rules/ide/the-server-is-synchronous.md, crates/nvs-lsp/tests/incremental.rs]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: file-system-counts
    stage: 1
    name: nvs check --count prints how many paths it resolved, files it read, directories it listed and settings files it parsed
    run: [cargo, test, --test, check_command, --, count_reports_file_system_work]
    timeout: 1800
    tests:
      - count_reports_file_system_work
  - id: index-work-bounded
    stage: 2
    name: a cold editor index resolves each path, and parses each settings file, a bounded number of times however many files the tree holds
    run: [cargo, test, --test, index, --, work_per_file]
    timeout: 1800
    tests:
      - an_index_resolves_each_files_path_a_bounded_number_of_times_per_file
      - an_index_parses_each_settings_file_once_until_it_changes
      - an_edit_to_nvs_toml_reaches_the_next_analysis
  - id: class-file-check
    stage: 3
    name: nvs check of a class file walks each program of the tree at most once and stops at its owner
    run: [cargo, test, --test, check_borrow, --, walks_each_program_once]
    timeout: 1800
    tests:
      - a_class_file_check_walks_each_program_once
  - id: parallel-index
    stage: 5
    name: the index settled on worker threads holds exactly what the serial index holds, and an edit during the build re-queues what it made stale
    run: [cargo, test, --test, incremental, --, parallel]
    timeout: 1800
    tests:
      - a_parallel_index_holds_what_the_serial_index_holds
      - an_edit_during_a_parallel_index_requeues_the_entries_that_read_it
      - a_settled_parallel_index_leaves_no_worker_running
---

# reading a whole project costs the compiler's work and little more

## Why

The user found the editor stalling at start on this repository (8,562 indexed `.nvs` files). Measured
on 2026-11-04 with temporary timers over stdio:

- The badge request `nvs/fileKinds` settled the whole index before it answered, so every request at
  start waited minutes. Fixed in `8bfd5e22b`, which also walks the tree once at start.
- A cold index took **145 s**. 101 s of it was `lenders::choose` resolving each file's path about 480
  times (once per lender, several times over; on Windows each resolution opens the file), and 9.9 s was
  index bookkeeping that scanned every indexed file twice per entry. Fixed in `92d208d21`: the cold index
  is now **23–29 s**.

What is left of a cold index, per phase over 8,409 analysed entries:

| Phase | Total | Where |
|---|---|---|
| reading the settings for each file (`extension_set`) | 5.0 s | `crates/nvs-lsp/src/document.rs:969` |
| the per-graph cache key (`graph_key`, which Debug-formats the whole extension set) | 3.1 s | `crates/nvs-lsp/src/document.rs:926` |
| the type check | 5.7 s | `crates/nvs-lsp/src/document.rs:741` |
| load and parse the entry | 2.6 s | same |
| resolve the program | 2.3 s | same |
| recording declarations and uses | 1.3 s | `crates/nvs-lsp/src/index.rs:1169` |
| choosing the lender | 1.25 s | `crates/nvs-hir/src/lenders.rs` |

For comparison, `nvs check` of one small file costs under 5 ms over the process's 55 ms start, and of
the whole registry app (`apps/registry/public/index.nvs`) about 70 ms. `nvs check` of one class file of
that app (`apps/registry/src/AccountPage.nvs`) costs about 400 ms, three times the whole app, because it
walks the programs under the root to find the one the file belongs to.

Stage 1 measured each whole-project path with `--count`'s file-system counts (debug binary; the
counts are the same on every machine, and the times are notes from the release binary):

| Path | resolved | read | listed | settings | programs | time |
|---|---|---|---|---|---|---|
| `nvs check apps/registry/src/AccountPage.nvs`, from this checkout's root | 8,857 | 120 | 4,550 | 1 | 3 | 0.39 s |
| the same, from `apps/registry` | 308 | 70 | 5 | 1 | 1 | |
| `nvs check apps/registry/public/index.nvs`, the whole app | 169 | 79 | 1 | 1 | 0 | |
| `nvs check packages/rest/tests/ClientTest.nvs`, from the root | 10,708 | 9,067 | 4,558 | 1 | 115 | |
| `nvs run --count examples/program.nvs`, uncached and cached alike | 23 | 6 | 1 | 1 | 0 | 0.06 s |
| `nvs check packages/rest/tests` (what `nvs test` of it compiles, 12 files) | 48 | 12 | 0 | 1 | 0 | 0.25 s |
| `nvs fmt --check docs/examples` (4,627 files) | 0 | 4,627 | one per directory | 0 | 0 | 0.73 s |

The editor on this checkout, with `bun nv lsp-start . --open apps/registry/src/AccountPage.nvs`
(release binary). The index's work is what its analyses and their filing did. The server's work is its
whole thread, the survey of the tree's programs included:

| Start | `initialize` | `documentSymbol` | index end | index work | server work |
|---|---|---|---|---|---|
| cold | 0.11 s | 2.02 s | 24.3 s | parsed 12,297, checked 13,111, resolved 134,092, read 9,106, listed 22, settings 592 | resolved 145,020, read 18,245, listed 4,581, settings 592, programs 116 |
| warm | 0.01 s | 0.96 s | 1.38 s | parsed 313, checked 320, resolved 4,044, read 295, listed 1, settings 5 | resolved 6,784, read 9,434, listed 4,560, settings 5, programs 116 |

A cold index resolves about 15 paths per file it reads and parses 592 settings files. A warm start
still walks all 116 programs and reads 9,434 files for the survey alone, because each walk reads the
library its programs share again.

A class file's check costs what it does because its lender search walks the tree `nvs check` runs
from, and each program walk reads the shared library again. An uncached compile resolves each
autoloaded file's path about three times. `nvs test` of a directory reads each file once more to look
for `require`. `nvs fmt` repeats nothing. Stage 3 has one item per repeated cost.

The shape of every cost found so far is one: work that is cheap once was repeated per file or per
program. That shape can be on any path that reads a whole project, so this goal measures each of them
before it fixes anything, and leaves a count-based test behind every fix so the cost cannot come back.
Then the remaining compiler work is spread over every core.

## Standing decisions

**Counts, not seconds** (`rule:testing/bench-counters`). Every guard this goal adds counts work: paths
resolved, files read, directories listed, settings files parsed, programs walked, entries analysed. A
count is the same on every machine, so a test can bound it per file. Wall time is printed by the
measuring command as a secondary note and is never asserted.

**Where the counts come from.** One relaxed atomic counter per kind, in the function that does the work:
`canonical_key` (`crates/nvs-diagnostics/src/source.rs:370`) and `requires::canonicalize`
(`crates/nvs-hir/src/requires.rs:742`) count resolutions; `SourceMap::load` counts files read; the
settings reader counts settings files parsed; `Lender::walk` counts programs walked. They feed
`nvs check --count`'s line (`crates/nvs-cli/src/main.rs:874`) and the editor index's `Work`
(`crates/nvs-lsp/src/document.rs:508`). The counters are a feature of `--count`, not a test hook. Cost:
one uncontended atomic add per syscall-backed call, next to a syscall that costs a thousand times more.
A test reads a delta around the work it runs, so tests in one binary running at once use a
per-thread counter: the counters are thread-local, summed where a command runs work on threads.

**The measuring command is `bun nv lsp-start`.** It starts `target/release/nvs lsp` on a root over stdio,
answers `window/workDoneProgress/create`, and prints: the time to the `initialize` answer, to the first
`documentSymbol` answer on a named file, and to the end of the indexing progress, then the index's
`Work` counts. `--cold` deletes that root's index file first (the file is under the data folder's
`cache/lsp-index/`, `crates/nvs-lsp/src/server.rs:687`). It is the stdio probe the 2026-11-04
measurement ran by hand twice (AGENTS.md rule 12).

**Settings are cached, and revalidated by modification time.** `extension_set`'s doc comment says it
reads the tree again on every analysis because "a cached set would be a second answer that outlives an
edit to `nvs.toml`". The cache keeps that promise: it is keyed by the analysed file's directory, and each
entry holds every settings file it read (and every one it looked for and did not find) with its size and
modification time. A lookup stats those paths and reuses the entry only when all of them match. Cost: one
entry per directory that holds an analysed file, for the session; a stat per settings path per analysis
instead of a read and a parse. A settings file written twice within the file system's time resolution
with the same size is the one case it misses, the same trade the index file's stamp already makes
(`rule:ide/the-index-is-incremental-and-persistent`). Safe fallback: if a case shows the cache serving a
stale set, drop the cache for that case and record why.

**The extension set is hashed once.** `graph_key` hashes `format!("{extensions:?}")` per analysis. The
settings cache stores each set with its digest, and `graph_key` takes the digest.

**Stage 3's items are written by stage 1.** Stage 1 measures four paths, each with `--count`'s new
counts: `nvs check` of `apps/registry/src/AccountPage.nvs`; an uncached compile, which is
`nvs cache clear` and then `nvs run --count` of a program under `examples/` that autoloads a library and
exits on its own; `nvs test` of a directory under `tests/`; and `nvs fmt --check` of `docs/examples`. It appends one item to stage 3 per
cost that repeats per file or per program, with its anchor, and writes "nothing repeated" for a path
that has none. The class-file check is known already: the first item.

**Parallel indexing design (stages 4 and 5).** The server stays synchronous in the sense
`rule:ide/the-server-is-synchronous` states: one thread owns the store, reads the channel and answers.
Indexing work moves to workers:

- **Workers**: `std::thread::available_parallelism() - 1`, at least 1, no setting. They exist only
  while entries are pending, and the last one exits when the queue empties, so a settled server has no
  worker thread (the user's "idle near zero").
- **What a worker owns**: its own parse and check session (`crates/nvs-lsp/src/session.rs`), a snapshot
  of the open buffers' text taken when the job was sent, the lenders (behind an `Arc`, read-only), and
  the stubs path. Nothing a worker reads is shared mutable state, so there are no locks on the analysis
  path. Cost: one session per worker for as long as entries are pending, freed when the queue empties.
  That spends memory to buy latency, which the priority ordering allows; the rule fragment states it.
- **What a worker returns**: the computed records of each file its analysis reached (today's `Indexed`)
  and the entry's reads, never the `Analysed` itself (it holds `Rc`).
- **Merging is in entry path order, on the main thread**, applying today's rules: the first analysis that
  reaches a file keeps it, and an entry whose file is already indexed and that owns nothing is skipped. A
  result that arrives ahead of an earlier entry's waits. So the index is byte for byte the serial one. An
  entry a worker analysed and the merge then skips is wasted work, bounded by the number of workers.
- **An edit while workers run**: every job carries the store's change counter. At merge, a result whose
  reads include a file changed since its counter is dropped, and its entry goes back to pending.
- **A whole-tree request** (`needs_every_file`, `crates/nvs-lsp/src/server.rs:666`) waits for the workers
  to drain, as it waits for `settle` today.
- **Progress** counts merged entries, so `crates/nvs-lsp/src/progress.rs` does not change.
- Stage 4 makes the split with no thread: the serial loop calls the new analysis function and the merge.
  Every existing index test stays green there before any thread exists.

**How stage 1 built them.** The counters are `nvs_diagnostics::work` (the crate every reader already
depends on), thread-local `Cell`s, cheaper than an atomic. `nvs_config`'s `Disk` counts its trust and
canonical checks, its listings and its settings reads. The index's `Work` has a `files` field. The
server reports the index's `Work` and its thread's counts in one `window/logMessage` after an
indexing run's `end` (`crates/nvs-lsp/src/progress.rs`), which is how `bun nv lsp-start` reads them.
`lsp-start` runs the server with its own data folder, `.agent-tmp/lsp-start/data/`, and `--cold`
deletes that folder's `cache/lsp-index/`: it never touches an editor's data folder, and needs no
BLAKE3 of the root to name the index file. `nvs test` and `nvs fmt` take no `--count`, so stage 1
measured them with `nvs check --count` per file and a read of their code.

**Declined:**

- **Parallel `nvs check` of a directory and `nvs test`.** Not measured as slow; a backlog item if stage
  1's numbers say otherwise.
- **A setting for the worker count.** Nobody asked for one, and a fixed rule is one less thing to tune.
- **Sharing one parse cache across workers behind a lock.** A lock on the analysis path costs latency on
  every analysis to save memory, the wrong direction.
- **Timing assertions.** `rule:testing/perf-two-mechanisms` keeps time out of pass/fail.

**Lanes.** Stage 2 edits `crates/nvs-lsp/src/document.rs`; stage 3 edits `crates/nvs-cli` and
`crates/nvs-hir`. They touch disjoint files after stage 1, so stage 3 may run as a side goal. Stages 4
and 5 edit the files stage 2 edits and run after it, in order.

## Stages

### 1. nvs check --count and the editor index report the file-system work they did, and every whole-project path is measured

**Does:** adds the file-system counters, prints them in `nvs check --count` and the index's `Work`, adds
`bun nv lsp-start`, then measures each whole-project path and writes stage 3's items.

**Split:** the four measurements of the last item go to one `reader` subagent each, in parallel, with the
command to run and the answer's shape (each count, per file, and the anchor of any count that grows faster
than the files). The session writes the items.

- [x] The thread-local counters, as § *Standing decisions* says, with a module doc saying what each
  counts and its cost. `crates/nvs-diagnostics/src/source.rs:370`, `crates/nvs-hir/src/requires.rs:742`
- [x] `nvs check --count` prints `resolved=`, `read=`, `listed=`, `settings=` and `programs=` after its
  compiler counts. `crates/nvs-cli/src/main.rs:874`
- [x] The test `count_reports_file_system_work`: a two-file program's counts are exact.
  `crates/nvs-cli/tests/check_command.rs:1`
- [x] The index's `Work` carries the same counts, summed per entry. `crates/nvs-lsp/src/document.rs:508`
- [x] `bun nv lsp-start <root> [--open <file>] [--cold]`, with its help text, as § *Standing decisions*
  says. `tools/nv/cmd/lsp-start.ts:1`
- [x] `rule:testing/bench-counters` names the new counts. `docs/rules/testing/bench-counters.md:27`. Then
  `bun nv rules --render` and `bun nv rules --check`.
- [x] Measure the four paths § *Standing decisions* names and the editor's cold and warm start with
  `bun nv lsp-start`. Write the figures into § *Why*, and one stage-3 item per repeated cost.

### 2. the editor reads each settings file once per change, and hashes each extension set once

**Does:** caches the settings read per directory, revalidated by modification time, and hands
`graph_key` the set's digest.

- [ ] The settings cache, keyed and revalidated as § *Standing decisions* says, held by `Documents`.
  The doc comment of `extension_set` is rewritten to say what it caches and why it is never stale.
  `crates/nvs-lsp/src/document.rs:969`
- [ ] `graph_key` takes the cached digest. `crates/nvs-lsp/src/document.rs:926`
- [ ] The three tests the `index-work-bounded` check names, over a scratch tree of many small files
  that share one `nvs.toml`. Each bounds a count per file; none reads a clock.
  `crates/nvs-lsp/tests/index.rs:1`
- [ ] `rule:ide/the-index-is-incremental-and-persistent` says what the cache costs.
  `docs/rules/ide/the-index-is-incremental-and-persistent.md:1`
- [ ] Run the `index-work-bounded` check's command once: green.
- [ ] `bun nv lsp-start . --cold` from this checkout's root: record the cold figures in § *Why*.

### 3. nvs check of a class file, an uncached compile, nvs test of a directory and nvs fmt of a tree learn each file-system fact once

**Does:** removes each repeated cost stage 1 found outside the editor.

- [ ] `nvs check` of a class file walks the programs under the root once each, in path order, and stops
  at the first that owns the file: find why it costs three times the whole app, and fix that.
  `crates/nvs-hir/src/lenders.rs:301`, `crates/nvs-cli/src/main.rs:1252`
- [ ] The test `a_class_file_check_walks_each_program_once`, reading `--count`'s `programs=`.
  `crates/nvs-cli/tests/check_borrow.rs:1`
- [ ] The lender search of a class file walks the project the file belongs to, not the directory
  `nvs check` runs from: from this checkout's root it lists 4,550 directories and resolves 8,857 paths
  for 120 files read, against 5 and 308 from `apps/registry`. Find the survey root
  (`crates/nvs-cli/src/main.rs:1438`) and bound it by the file's project. `crates/nvs-hir/src/lenders.rs:343`
- [ ] `first_lender` resolves every `.nvs` path under the root before it walks one program; resolve a
  path only when its program is walked. `crates/nvs-hir/src/lenders.rs:303`
- [ ] Each `Lender::walk` starts from an empty source map, so the library the programs share is read
  and parsed again per program (115 programs and 9,067 files read for one test file of
  `packages/rest/tests`). Share one map across the walks of one search. The editor's survey walks the
  same way at every start, warm included (9,434 files read), so `Documents::survey` takes the shared
  map too. `crates/nvs-hir/src/lenders.rs:69`, `crates/nvs-lsp/src/document.rs:304`
- [ ] Once a lender is found, the class file is loaded and checked a second time with a fresh map.
  Keep the first load. `crates/nvs-cli/src/main.rs:1260`
- [ ] An autoloaded file's path is resolved by the directory walk, again by the name probe, and a third
  time when claims are compared: about two resolutions per extra file on an uncached compile. Resolve
  it once and carry the result. `crates/nvs-hir/src/autoload.rs:871`, `:560`, `:880`
- [ ] The run's cache key resolves the entry path the compile already resolved, and again on each
  validation. `crates/nvs-cli/src/script.rs:1436`, `:1744`, `:1960`
- [ ] `nvs test <dir>` reads every file once only to look for `require`, then reads it again to
  compile; and with a `require` present it resolves the whole directory twice (`unrequired`). Learn
  both from the one compile. `crates/nvs-cli/src/main.rs:3337`, `:3348`
- [ ] `nvs test <dir>` resolves each file's path in `unrequired` and again after the compile.
  `crates/nvs-cli/src/main.rs:3363`, `:3213`
- [ ] Each fix above adds or tightens a count test that would go red if the work came back.
- [ ] `nvs fmt --check` of a tree: nothing repeated (one listing per directory, one read per file, no
  settings and no resolution).
- [ ] Run the `class-file-check` check's command once: green.

### 4. an index entry is analysed by a function that owns everything it reads, and merged into the index apart from that

**Does:** splits `absorb` into an analysis that returns the records and a merge, with no thread yet.

- [ ] A snapshot type with what one analysis reads: the open buffers' text, the lenders behind an
  `Arc`, the stubs path, and a session it owns. It is `Send`. `crates/nvs-lsp/src/document.rs:741`
- [ ] `absorb` becomes the analysis (returning each reached file's `Indexed` and the entry's reads) and
  the merge (today's first-reach rule). `settle_one` calls both. `crates/nvs-lsp/src/index.rs:1169`
- [ ] Run `cargo test --test index --test incremental --test autoload`: green, unchanged.

### 5. the editor settles its index on every core, and the index it builds is the serial one

**Does:** runs the stage-4 analysis on workers, merges in path order, re-queues what an edit made stale.

- [ ] The worker pool and the ordered merge, as § *Standing decisions* says. The serve loop's idle arm
  sends jobs and merges results. `crates/nvs-lsp/src/server.rs:334`
- [ ] A whole-tree request drains the workers first. `crates/nvs-lsp/src/server.rs:666`
- [ ] The three tests the `parallel-index` check names. The equality test builds one tree serially and in
  parallel and compares the saved index bytes. `crates/nvs-lsp/tests/incremental.rs:1`
- [ ] `rule:ide/the-server-is-synchronous` says which work runs on workers, what each worker holds and
  for how long, and why the answer is still the one thread's.
  `docs/rules/ide/the-server-is-synchronous.md:1`. Then `bun nv rules --render` and
  `bun nv rules --check`.
- [ ] Run the `parallel-index` check's command once: green.
- [ ] `bun nv lsp-start` cold and warm on this checkout: record the figures in § *Why*.

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is done and its check `file-system-counts` is green. The five counters are
`nvs_diagnostics::work` (`crates/nvs-diagnostics/src/work.rs`), printed by `--count` and summed in the
index's `Work.files`. `bun nv lsp-start` measures the editor (`tools/nv/cmd/lsp-start.ts`). § *Why*
has every path's figures, and stage 3 has one item per repeated cost. § *Standing decisions* § *How
stage 1 built them* records what differs from the plan.

The acceptance sweep's reds were stale: `env-secret-proofs` is defined nowhere in the tree (it comes
from looper's service), and nightly 41813182715 ran on `f3150b922`, which has neither `a2f451fad`
(Miri) nor `a8bdf37fb` (the macOS cli test).

Not run, so may be red at the gate: `crates/nvs-lsp/tests/progress.rs` (an indexing run's `end` is
now followed by a `window/logMessage`; its reader ignores other methods, so it should hold), and
`incremental.rs`'s three `Work` assertions, which now take `files: work.files`.

### Next

Stage 2, from its first item: the settings cache in `crates/nvs-lsp/src/document.rs:969`
(`extension_set`, `program_tree` at `:1020`), then `graph_key` at `:926`. A cold index parses 592
settings files and resolves about 15 paths per file read (§ *Why*), and the new tests in
`crates/nvs-lsp/tests/index.rs` read `Work.files.settings` and `.resolved` per file.
`bun nv lsp-start . --cold` gives the before and after; run `bun nv relink --build` first.
Stage 3 may run as a side goal: its items now touch `crates/nvs-lsp/src/document.rs:304`
(`Documents::survey`) too, so a side lane takes that item after stage 2 lands.

### Backlog
