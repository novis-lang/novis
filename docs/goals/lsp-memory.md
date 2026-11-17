---
looper: 1
title: "the language server checks each file once, whatever the number of entries and cores"
summary: "Reached when a settle of the index parses and checks each file once however many entry files read it and however many workers run, every worker reads one shared parse of each file, a keystroke in an open document checks that document's bodies alone, and bun nv lsp-start prints the peak memory of a cold index and of an edit."
context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/workers.rs, crates/nvs-lsp/src/document.rs, crates/nvs-lsp/src/session.rs, crates/nvs-lsp/src/parse_cache.rs, crates/nvs-lsp/tests/incremental.rs, docs/rules/ide/the-server-is-synchronous.md]
stages:
  - id: 1
    title: bun nv lsp-start measures memory on a generated tree, and the count tests are written red
    status: done
    context: [tools/nv/cmd/lsp-start.ts, crates/nvs-lsp/tests/incremental.rs]
  - id: 2
    title: an analysis checks the bodies of the files it will own and no others
    status: open
    context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/document.rs, crates/nvs-lsp/src/session.rs]
  - id: 3
    title: the workers split one graph's bodies between them, and never check one graph twice
    status: open
    context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/workers.rs]
  - id: 4
    title: every worker reads the server's one parse of each file
    status: open
    context: [crates/nvs-lsp/src/parse_cache.rs, crates/nvs-lsp/src/workers.rs, crates/nvs-lsp/src/session.rs]
  - id: 5
    title: the rule and the module docs say what the index spends, with the measured figures
    status: open
    context: [docs/rules/ide/the-server-is-synchronous.md, crates/nvs-lsp/src/workers.rs, crates/nvs-lsp/src/session.rs, crates/nvs-lsp/src/parse_cache.rs]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: once
    stage: 3
    name: "a cold settle and a surface edit read by every program check each file's bodies once, on the server thread and on four workers, and a keystroke in an open library file checks its own bodies alone"
    run: [cargo, test, --test, incremental, --, a_cold_settle_checks_each_file_once_on_any_number_of_workers, a_surface_edit_read_by_every_program_checks_each_file_once, a_keystroke_in_an_open_library_file_checks_its_own_bodies_alone, a_parallel_index_holds_what_the_serial_index_holds]
    timeout: 1800
    tests:
      - a_cold_settle_checks_each_file_once_on_any_number_of_workers
      - a_surface_edit_read_by_every_program_checks_each_file_once
      - a_keystroke_in_an_open_library_file_checks_its_own_bodies_alone
      - a_parallel_index_holds_what_the_serial_index_holds
  - id: parsed-once
    stage: 4
    name: "a cold settle parses each file once on any number of workers"
    run: [cargo, test, --test, incremental, --, a_cold_settle_parses_each_file_once_on_any_number_of_workers, a_parallel_index_holds_what_the_serial_index_holds]
    timeout: 1800
    tests:
      - a_cold_settle_parses_each_file_once_on_any_number_of_workers
      - a_parallel_index_holds_what_the_serial_index_holds
---

# the language server checks each file once, whatever the number of entries and cores

## Why

A user reported that the language server of a project of about 1000 files held 2.4 GB. Another user's
machine froze when `nvs lsp` reached 4.2 GB beside a loaded `nvs serve`. Measured on 2026-11-17 with the
release binary on a 16-core machine, on 972 files made from 12 copies of `apps/registry/src`, one
namespace per copy, 7.1 MB of source, and 12 entry files that each autoload `App` from `../src`:

| Moment | Memory (private bytes) | Work |
|---|---|---|
| warm start, every entry from the index file | 165 MB | `parsed=961 checked=961` |
| cold index | **5.4 GB peak**, 450 MB once settled, 18 s | `parsed=11630 checked=11630`, 15 workers |
| one edit that adds a method to an open file | **4.5 to 4.8 GB for about 25 s**, 80 CPU seconds | 9 workers |
| the same, once the workers stop | 843 MB | |
| forty such edits in a row | flat at 4.8 GB, about 2 s an edit | |

It is not a leak: memory falls once the workers stop. Three causes multiply.

1. **Every entry checks its whole graph.** `Core\Router::dispatch` finds routes across every class
   under the autoload root, so each of the 12 entries' graphs is all 972 files. `analyse_through`
   (`crates/nvs-lsp/src/document.rs:1046`) checks every body whose key moved, and the key hashes the
   graph key (`crates/nvs-lsp/src/session.rs:128` `keys`, `document.rs:1282` `graph_key`), which
   differs per entry. So every entry checks all 972 bodies, and `merge`
   (`crates/nvs-lsp/src/index.rs:1426`) then throws away every record of a file an earlier entry
   indexed. 11630 checks for 972 files.
2. **Every worker keeps its own copy.** `work` (`crates/nvs-lsp/src/workers.rs:241`) gives each worker a
   `Session` of its own, so each holds a parse and a check of the whole graph, about 400 MB here.
   There are `cores - 1` workers, so memory grows with the core count times the project.
3. **One surface edit queues every reader.** `refresh` (`index.rs:693`) queues the 12 entries that read
   the edited file, and each checks all 972 bodies again on the workers.

The rule `rule:ide/the-server-is-synchronous` says a worker costs "an analysis an earlier entry's result
made unneeded, at most one per worker". The measurement shows each such analysis is the whole project.

## Standing decisions

**The fix (the person's call, from the measurement).** Each file's bodies are checked once per
settle, whatever the number of entries that read it and the number of workers. Every worker reads one
parse of each file. A keystroke checks the bodies of the document it edits. This saves CPU and editing
latency as well as memory, so it is not a trade against the request path.

**A body is checked by the analysis that will own its record (my call).** `analyse_entry`
(`index.rs:1517`) already takes `indexed`. The bodies it checks are `keys.bodies` minus every file
`indexed` holds, plus the entry itself. A file whose bodies were not checked has no key stored
(`session.rs:148` `store`), so a later analysis that needs it checks it. The records of skipped files are
not built. `merge` discarded them anyway. An open document's analysis (`absorb`, `index.rs:1400`) is the
same call, so a keystroke checks the document's own bodies once every other file is indexed.

**A file's checked output depends on the graph's declarations alone (my call, proved in stage 2).**
The session already reuses one file's output while another file's body changes. So the check of a
file's bodies reads the declarations of its graph and the count of anonymous functions before it, and
no other body. Stage 2 proves it with
`a_cached_analysis_reports_what_a_fresh_one_does_after_every_kind_of_edit` and
`a_parallel_index_holds_what_the_serial_index_holds`. If a body turns out to read another body, the
stage writes that down in the handoff, and the reading file's bodies are checked with the file it reads.

**The workers split one graph, they do not run graphs side by side (my call).** Today `turns`
(`index.rs:893`) sends whole entries ahead of their turn, each with the `indexed` set of the moment it
was sent. That set is nearly empty during a cold index, so every worker checks everything. Instead:

- A job names an entry and the files whose bodies it checks: its **part**.
- The server thread plans the entry at the head of `pending`. Its graph is the read set the index kept
  for it (`reads`), else the one the index file recorded, else unknown.
- An unknown graph is planned by a **plan job**: a worker resolves the entry's graph, which parses and
  reads declarations and checks no body, and returns the file list. The plan job's parses go to the
  shared parses of stage 4.
- The graph's files that no entry indexed and no job in flight claims are split into as many parts as
  the pool has workers, in path order. Each part is one job. The claims are kept on the server thread
  beside `in_flight`, so nothing shared is locked.
- The next entry is planned only with the files the earlier one claimed left out. So an entry whose
  graph another entry already covers checks only its own file.
- The results are filed at the entry's turn, in part order. Each file's record is the same whatever
  part checked it, so the index is the serial one, byte for byte, as the rule says now.

**The parses are shared without a lock (my call).** The rule declines "one parse cache shared by every
worker, behind a lock", because a lock on the analysis path costs latency. That reason holds, and no
lock is added. The server thread keeps the one `ParseCache`. A job carries an `Arc` of a read-only map
from each file of its part's graph to that file's `Arc<Vec<Stmt>>` (`parse_cache.rs:89`), which is
already shared. A worker parses only what the map lacks and returns those parses with its result. The
server thread files them. Every file the survey found is pinned to its `SourceId` on the server thread
before any job is sent, so a parse made on one thread is valid on every other. A file outside the tree
is parsed by the worker that loads it and is not shared.

**A worker keeps its session between jobs (my call).** It holds the declarations of the graphs it
planned or checked and the checked output of its parts, until the pool stops. That is one copy of the
project's checked output spread over all the workers, where today it is one copy per worker.

**What is not done (my call).** No worker cap and no setting for the worker count: a cap hides the
cause, and the rule declines a setting. No finer surface key: `crate::session`'s decision declines it,
and after this goal a surface edit checks each file once, not once per reader. The per-file size of a
parse and a check, about 55 times the source text, is the next cost. It is measured here and left to a
later goal.

**What it costs.** One parse per file of the tree for the server's life, which the server thread
already keeps. While entries wait, each worker holds the declarations of one graph and its share of
the checked output. A plan job is one extra resolve of a graph nobody indexed yet. A job carries one
`Arc` per graph file.

**Lanes.** Stages 2 to 4 change `index.rs` and `workers.rs` and run in order. Stage 1's
`tools/nv/cmd/lsp-start.ts` work is disjoint and may run as a side goal. Stage 5 needs stage 4.

**An edit's re-index shows a progress run (my call, stage 1).** `bun nv lsp-start --edits` waits for the
`index work:` line, which the server logged only at the end of the start's run. Now an edit that queues
entries after that run ended begins a run of its own (`crates/nvs-lsp/src/progress.rs` `track`), and
its `end` logs the line. A client sees the index catch up after a surface edit, which it did not before.
It costs one token per such edit, and is pinned by
`an_edit_that_queues_entries_after_the_start_begins_a_run_of_its_own` in `tests/progress.rs`.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. bun nv lsp-start measures memory on a generated tree, and the count tests are written red

**Does:** makes the measurement of § *Why* one command, and writes the count tests the checks name.
Nothing in the server changes yet.

- [x] `tools/nv/cmd/lsp-start.ts:1`: `--copies <n>` writes a tree of `n` copies of
  `apps/registry/src` under `.agent-tmp/lsp-start/tree/`, each under `src/C<k>/` in namespace `App\C<k>`,
  with one entry `public/c<k>.nvs` per copy that autoloads `App` from `../src`, and `apps/registry/nvs.toml`.
  Without a root, it measures that tree.
- [x] The same file: after the indexing end, it prints the process's private bytes now and at their
  peak. On Windows that is `Get-Process`'s `PrivateMemorySize64` and `PeakPagedMemorySize64`, on Linux
  `/proc/<pid>/status`'s `VmRSS` and `VmHWM`. `--edits <n>` then opens the first entry's graph's
  `Publish.nvs`, adds one method per edit, waits for each `documentSymbol` answer, waits until the
  server logs its next `index work:` line, and prints both figures again. The header comment says so.
- [x] Run `bun nv lsp-start --copies 12 --cold --edits 5` once on the release binary, and write its
  figures into the handoff as the baseline.
- [x] `crates/nvs-lsp/tests/incremental.rs:1025`: beside `planned`, the four tests the `once` and
  `parsed-once` checks name that do not exist yet, on `tree(12, 8)`. Each settles with `planned(dir, 0)`
  and `planned(dir, 4)` and reads `take_work`. The cold tests expect `checked` and `parsed` to equal
  the number of distinct files. The surface-edit test edits a library class's signature and expects
  `checked` to equal the files. The keystroke test opens a library file, edits a body, and expects
  `checked: 1`. They fail now, and the handoff names them as expected red.

### 2. an analysis checks the bodies of the files it will own and no others

**Does:** the first of the three causes, on the server thread and on the workers as they are now.

- [ ] `crates/nvs-lsp/src/index.rs:1517` `analyse_entry` passes `indexed` down to `analyse_through`
  (`crates/nvs-lsp/src/document.rs:1046`), which removes every indexed file but the entry from
  `keys.bodies` before `check_program_partial` (`:1212`), and stores keys only for the files it checked.
  The record loop (`index.rs:1539`) skips the files whose bodies were not checked.
- [ ] `analyse` (`document.rs:972`), the open document's analysis outside the index, passes "every file
  but this one", so a keystroke checks one file's bodies once the index holds the rest.
- [ ] Run `a_cached_analysis_reports_what_a_fresh_one_does_after_every_kind_of_edit`,
  `a_parallel_index_holds_what_the_serial_index_holds` and
  `a_keystroke_in_an_open_library_file_checks_its_own_bodies_alone` once. If the first two are red
  because a body reads another body, the handoff says which, as § *Standing decisions* says.
- [ ] Doc comments of `analyse_entry`, `analyse_through` and `crate::session` say what is true now.

### 3. the workers split one graph's bodies between them, and never check one graph twice

**Does:** the second cause's multiplier: plan jobs, parts and claims, as § *Standing decisions* says.

- [ ] `crates/nvs-lsp/src/workers.rs:41` `Job` gains the part, the files whose bodies it checks, and a
  plan kind that checks no body and returns the graph's file list. `Done` (`:58`) carries the part and,
  for a plan, the list.
- [ ] `crates/nvs-lsp/src/index.rs` `turns` (`:893`) and `send` (`:950`) plan the head entry, split
  its unclaimed files into parts, keep the claims beside `in_flight`, and send the next entry only with
  claimed files left out. `file_result` (`:975`) files an entry's parts in part order at its turn. A stale part
  is sent again, alone.
- [ ] `a_cold_settle_checks_each_file_once_on_any_number_of_workers` and
  `a_surface_edit_read_by_every_program_checks_each_file_once` run once, then the `once` check.

### 4. every worker reads the server's one parse of each file

**Does:** the shared parses, with no lock.

- [ ] `crates/nvs-lsp/src/parse_cache.rs:61` `ParseCache` pins every file the survey found before the
  first job, gives a read-only `Arc` map of a graph's parses for a job, and files the parses a worker
  returns. Its module doc's *What it costs* says so.
- [ ] `crates/nvs-lsp/src/workers.rs:241` `work`: the worker's session reads parses from the job's map
  first, and returns the ones it made.
- [ ] The `parsed-once` check runs once.

### 5. the rule and the module docs say what the index spends, with the measured figures

- [ ] `docs/rules/ide/the-server-is-synchronous.md:18`: the paragraph about workers says that a job is
  a part of one graph, that each body is checked once per settle, and that every worker reads the
  server's parses through the job. Its cost sentence gives the measured figures. The Declined entry
  "One parse cache shared by every worker, behind a lock" stays, and a new entry declines a worker cap
  with the reason above. `bun nv rules --render`, then `bun nv rules --check` is green.
- [ ] `crates/nvs-lsp/src/workers.rs:1` and `crates/nvs-lsp/src/session.rs:1` module docs say the same,
  with the numbers.
- [ ] Rebuild the release binary with `bun nv relink --build` and run
  `bun nv lsp-start --copies 12 --cold --edits 5` once. Write the figures beside the baseline in the
  handoff. The cold peak and the edit peak are each expected under 1 GB. A figure above that names in
  the handoff what holds the memory, measured.

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is done. `bun nv lsp-start --copies <n> --edits <m>` measures memory after indexing and edits.
Waiting for an edit's `index work:` line needed the progress change in § *Standing decisions*.

The baseline, `bun nv lsp-start --copies 12 --cold --edits 5` on the release binary, Windows, 16 cores
(the work counts are summed since the start):

| Moment | Memory | Work |
|---|---|---|
| cold index, 15.6 s | private 467 MB, **peak paged 5752 MB** | `parsed=11630 checked=11630` |
| after 5 edits that each add a method to `src/C1/Publish.nvs`, 98.5 s | private 1586 MB, peak 5752 MB | `parsed=69326 checked=69476` |

So each edit parsed and checked about 11.5k files, nearly a cold index's worth, for one file of 972.
The four count tests are at the end of `crates/nvs-lsp/tests/incremental.rs`, on `tree(12, 8)`, 44
files. Expected red until their stage:

- `a_cold_settle_checks_each_file_once_on_any_number_of_workers`: 132 checked on the server thread.
- `a_cold_settle_parses_each_file_once_on_any_number_of_workers`: 44 on the server thread, 116 on four
  workers.
- `a_surface_edit_read_by_every_program_checks_each_file_once`: 142 checked on the server thread.
- `a_keystroke_in_an_open_library_file_checks_its_own_bodies_alone` passes already, on both. It stays
  as the guard that stages 2 to 4 keep a keystroke at one file.

The sweep's reds need no change: `env-secret-proofs` is stale, and `0fe2fdef25` removed the `chrono`
that failed the nightly `supply chain`.

### Next

Stage 2 from its first item: `analyse_entry` (`crates/nvs-lsp/src/index.rs:1517`) passes `indexed` to
`analyse_through` (`crates/nvs-lsp/src/document.rs:1046`), which drops indexed files but the entry
from `keys.bodies` before `check_program_partial` (`document.rs:1212`). Then `analyse`
(`document.rs:972`) and the record loop (`index.rs:1539`). Rerun the cold-check test once by name: it
should reach 44 on the server thread after stage 2.

### Backlog

- The per-file cost of a parse and a check, about 55 times the source text on the tree above, for a
  later goal.
