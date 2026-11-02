---
looper: 1
title: "the language server keeps a persistent incremental index, so a keystroke costs what changed and not the project"
summary: "a body-only keystroke in a library file of a tree with many programs parses one file and checks one file's bodies, a cold start with a valid on-disk index analyses only the open document's entry, and the editor reports what nvs check reports for the open file."
context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/document.rs, docs/rules/ide/the-server-is-synchronous.md, docs/rules/ide/five-features-are-one-reference-index.md]
stages:
  - id: 1
    title: a generated perf fixture and tests that count the files parsed and the bodies checked
    status: done
    context: [crates/nvs-lsp/tests/latency.rs, crates/nvs-lsp/tests/autoload.rs, crates/nvs-lsp/src/server.rs]
  - id: 2
    title: "a file is parsed again only when its text changed: the loader takes a cached parse"
    status: done
    context: [crates/nvs-hir/src/requires.rs, docs/rules/ide/one-grammar-one-tree.md]
  - id: 3
    title: "the checker checks the bodies of a chosen set of files and folds every other file's cached output"
    status: done
    context: [crates/nvs-types/src/check.rs, crates/nvs-types/src/expr_table.rs, crates/nvs-diagnostics/src/source.rs, crates/nvs-types/tests/common/mod.rs]
  - id: 4
    title: "each file has a declaration-surface hash, and a body-only edit re-queues no reader"
    status: done
    context: [crates/nvs-lsp/src/server.rs]
  - id: 5
    title: "the editor checks only the bodies whose inputs changed, against one session interner"
    status: open
    context: [crates/nvs-types/src/check.rs, crates/nvs-types/src/ty.rs, crates/nvs-diagnostics/src/source.rs]
  - id: 6
    title: "the index is written to disk and loaded at start, and only stale entries are analysed"
    status: open
    context: [crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/settings.rs, crates/nvs-config/src/data.rs, crates/nvs-config/src/cache.rs, docs/rules/ide/dependencies-are-allowlisted.md]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
    context: [docs/rules/ide/a-full-reanalysis-stays-under-a-bound.md, docs/agent/doc-style.md]
checks:
  - id: lsp-work-counted
    stage: 1
    name: an analysis and a settled refresh report the files they parsed and the files whose bodies they checked
    run: [bun, nv, verify, --fast, -p, nvs-lsp]
    timeout: 1800
    tests:
      - the_generated_tree_has_the_programs_and_the_library_it_was_asked_for
      - a_keystroke_counts_the_files_it_parsed_and_the_bodies_it_checked
  - id: hir-parse-cache
    stage: 2
    name: the program walk takes a parse it is handed and parses no file it was given
    run: [bun, nv, verify, --fast, -p, nvs-hir]
    timeout: 1800
    tests:
      - a_walk_takes_a_cached_parse_and_parses_no_file_it_was_given
      - a_cached_parse_replays_its_parse_diagnostics
  - id: lsp-parse-cache
    stage: 2
    name: an unchanged file is parsed once across keystrokes
    run: [bun, nv, verify, --fast, -p, nvs-lsp]
    timeout: 1800
    tests:
      - an_unchanged_file_is_parsed_once_across_keystrokes
      - a_file_whose_text_changed_on_disk_is_parsed_again
  - id: types-partial-check
    stage: 3
    name: a check of a body set with every other file's cached output reports and records what a whole check does
    run: [bun, nv, verify, --fast, -p, nvs-types]
    timeout: 1800
    tests:
      - a_partial_check_with_cached_outputs_matches_a_whole_check
      - a_route_collision_with_an_unchecked_file_is_still_reported
      - a_link_to_a_route_in_an_unchecked_file_still_resolves
      - a_cached_output_is_valid_in_a_second_source_map_with_the_same_pins
      - a_file_whose_starting_anonymous_function_count_moved_is_checked_again
  - id: lsp-surface
    stage: 4
    name: a body-only edit keeps a file's surface hash, a declaration edit changes it, and only a changed surface re-queues readers
    run: [bun, nv, verify, --fast, -p, nvs-lsp]
    timeout: 1800
    tests:
      - a_body_only_edit_keeps_the_surface_hash
      - a_signature_constant_attribute_or_require_edit_changes_the_surface_hash
      - a_body_only_keystroke_re_queues_no_reader
      - a_surface_edit_re_queues_the_entries_that_read_the_file
  - id: lsp-incremental
    stage: 5
    name: a body-only keystroke in a library file parses one file and checks one file's bodies whatever the number of programs, and a cached analysis reports what a fresh one does
    run: [bun, nv, verify, --fast, -p, nvs-lsp]
    timeout: 1800
    tests:
      - a_body_only_keystroke_in_a_library_file_parses_one_file_and_checks_one_files_bodies
      - a_body_only_keystroke_costs_the_same_with_two_programs_and_with_twenty
      - a_cached_analysis_reports_what_a_fresh_one_does_after_every_kind_of_edit
      - the_session_interner_is_rebuilt_once_it_doubles
  - id: lsp-disk-index
    stage: 6
    name: a cold start with a valid on-disk index analyses only the open document's entry, and a stale, foreign or broken index file is never trusted
    run: [bun, nv, verify, --fast, -p, nvs-lsp]
    timeout: 1800
    tests:
      - a_cold_start_with_a_valid_index_analyses_only_the_open_documents_entry
      - a_file_changed_on_disk_queues_its_entry_again
      - a_surface_changed_on_disk_queues_every_entry_that_read_it
      - a_build_stamp_mismatch_discards_the_index_file
      - a_format_version_mismatch_discards_the_index_file
      - a_deleted_file_leaves_the_loaded_index
      - a_truncated_or_corrupt_index_file_is_discarded
      - two_servers_writing_one_root_leave_one_whole_index_file
  - id: lsp-green
    stage: 7
    name: every nvs-lsp test, every .lspt case and the latency guard included, is green
    run: [bun, nv, verify, --fast, -p, nvs-lsp]
    timeout: 1800
    reach: true
    floor: false
  - id: owners-closes
    stage: 7
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, incremental-index]
    reach: true
    floor: false
---

# the language server keeps a persistent incremental index, so a keystroke costs what changed and not the project

## Why

The language server (`crates/nvs-lsp`) must stay fast whatever the size of the project, as an IDE with
a persistent index does. Every file already analysed is kept in memory and in an index on disk, so a
keystroke processes again only what changed, and a cold start never analyses the whole project.

What is true now, measured on a tree of 3 programs that autoload one shared library (about 220 `.nvs`
files) in the `proof` profile:

- One analysis (`analyse_over`, `crates/nvs-lsp/src/document.rs:676`) loads the whole graph of its
  entry: about 125 files, 110 of the library and 15 built-in component sources. Resolve takes about
  31 ms: every file of the graph is read and parsed, then `nvs_hir::resolve_program_linted` runs.
  Check takes about 38 ms: `check_program_loaded` (`crates/nvs-types/src/check.rs:158`) checks the
  bodies of every file. About 70 ms in all, up to 130 ms under load.
- `SymbolIndex` (`crates/nvs-lsp/src/index.rs:386`) queues entries in `pending` and the server loop
  settles one per idle turn (`crates/nvs-lsp/src/server.rs:300`). The first completion takes 0.3 to
  0.7 s. A keystroke takes 120 to 470 ms: one analysis of the open document, plus at most one queued
  analysis in flight, which is never interrupted (`rule:ide/the-server-is-synchronous`).
- Every analysis parses and checks the whole graph again, though one file changed and only the
  entry's results are published. `refresh` (`crates/nvs-lsp/src/index.rs:484`) queues every entry
  that read the changed file, whatever the edit was. So the work of one keystroke in a library file
  grows with the number of programs that read it, and each of those analyses grows with the library.

This goal makes the cost of a keystroke depend on what changed: the edited file is parsed and its
bodies are checked, and nothing else is, unless what other files can see of it changed.

## Standing decisions

**The landed work this goal builds on.** `extension_set` (`crates/nvs-lsp/src/document.rs:842`)
includes the built-in components. `SymbolIndex` has `pending`, `planned`, `settle_one` and `settle`.
`refresh` analyses open readers at once and queues the rest, `needs_every_file`
(`crates/nvs-lsp/src/server.rs:576`) names the requests that settle first, and `settled`
(`crates/nvs-lsp/src/server.rs:601`) re-checks completion files and asks for a lens refresh. An entry
whose own file is indexed and that owns no file is skipped. The gate that this holds is
`SymbolIndex::pending` at `crates/nvs-lsp/src/index.rs:418`. Nothing here is redone.

**Work is counted, never timed.** The guard tests count two numbers: the files an analysis parsed and
the files whose bodies it checked. `Analysed` gains `work: Work { parsed, checked }` for its own
analysis, and `SymbolIndex::work()` returns the sum over every analysis it ran since the last
`take_work()`. The counters live in the analysis and the index, never in a global, so two tests never
share one. A keystroke's count is the open document's analysis plus every entry its `refresh` queued,
settled until `pending` is empty. Queued work is deferred work, and a count that left it out would
pass while the editor still did it all. `crates/nvs-lsp/tests/latency.rs` keeps its timing guard for a
whole analysis.

**The fixture is generated, in `crates/nvs-lsp/tests/incremental.rs`.** `tree(programs, library)`
writes, into a `TempDir` built the way `crates/nvs-lsp/tests/autoload.rs:26` builds one, `programs`
programs (`Blog`, `Shop`, then `App3`, `App4` and on) that each autoload one shared library
`lib/Framework/`. The library has `library` classes in its own namespace, each with methods that call
another class's methods, two routes and one command, so the routes, commands and links tables have
rows across files. Each program has a `main.nvs` and a few files that call into the library. No
fixture is kept under `tests/lsp/`: its only property is its shape, and a file nobody reads rots
(the reason `latency.rs:12` gives). The counted tests use `tree(2, 40)` and `tree(20, 40)`, so the
debug build stays quick.

**1. A file is parsed again only when its text changed.** `nvs-lsp` keeps a `ParseCache`: one entry
per canonical path (`nvs_diagnostics::canonical_key`) with the BLAKE3 hash of its text
(`nvs_config::cache::content_hash`, `crates/nvs-config/src/cache.rs:290`), its statements as
`Arc<Vec<Stmt>>`, its parse diagnostics and its `check_declarations` diagnostics. The `nvs_hir` walk
(`crates/nvs-hir/src/requires.rs:302`) takes an optional `&mut dyn Parses` whose `parse(&SourceFile,
&mut Diagnostics) -> Arc<Vec<Stmt>>` replaces each `parse_file` call of the walk
(`crates/nvs-hir/src/requires.rs:500`, `:549`, `:571`, `:597`). The cache replays a hit's stored
diagnostics. `Loaded::stmts` (`crates/nvs-hir/src/requires.rs:155`) becomes `Arc<Vec<Stmt>>`. Where a
pass changes a file's statements, it calls `Arc::make_mut` there, and the stage says which sites do.
The safe fallback, if sharing proves wrong: the cache returns a clone of its `Vec<Stmt>`, and the
stage records the measured cost of that clone against a parse. `nvs check` passes no cache, and its
walk is unchanged. The file is still read on every analysis, because `SourceMap::load` reads it to
hash it. A read of 125 small files is not the measured cost, so a stat-based skip of the read is not
built. This is not the incremental reparse `rule:ide/one-grammar-one-tree` gave up: an edited file is
still parsed whole. The cache parses every file with the lossless `nvs_syntax::parse` and keeps its
trivia and syntax index too, because any file it holds may be the next analysis's entry, and a
reader's entry parsed outside the cache would count a parse per queued reader. That spends the trivia
and index of every file, which memory is last to pay for.

**2. The checker checks a body set and folds cached outputs.** `nvs-types` gains `check_program_partial(files,
..., bodies: &FxHashSet<SourceId>, cache: &mut CheckCache)`. `check_program_loaded` stays the whole-program
call, implemented as `check_program_partial` with every file in `bodies` and an empty cache, so `nvs
check`, `nvs-ir` and the tests keep their call and there is one code path. Signatures and the
whole-graph tables (`build_signatures`, the enum, const, attribute, deprecation and test-code tables,
`crates/nvs-types/src/check.rs:173-185`) are built for the whole graph every time, from the cached
parses. The per-file walk (`crates/nvs-types/src/check.rs:224-288`) writes each file into a
`FileOutput`: its own `ExprTypeTable` part, its diagnostics, its rows of `routes`, `commands`, `links`,
`codec_sites`, `row_sites`, `json_sites`, `decode_sites` and `encode_sites`, and the `anon_fn_seq` it
started at and ended at. A file outside `bodies` takes its `FileOutput` from `CheckCache`. Then the
rows of every file are folded in file order, and every pass after the walk runs over the whole folded
result as it does now (`crates/nvs-types/src/check.rs:289-329`): `routes::check_*`,
`commands::check_table`, `test_inherit::merge`, `record_pipeline_slots`, `links::resolve`, the
`derive` passes and `callables::resolve`. A `FileOutput` is stored before those passes write into the
table, so a cached part never carries a write that a later pass would make again. The stage measures
the cost of the passes after the walk on `tree(20, 40)` and writes the figure in the doc comment of
`check_program_partial`. `nvs-types` decides nothing about validity: the caller says which files are
in `bodies`.

**The anonymous-function counter is part of a cached output's key.** `anon_fn_seq` threads across
files (`crates/nvs-types/src/check.rs:190`), so a `FileOutput` is valid only where its file starts at
the same count. A file whose start moved is put in `bodies`. The labels do not change.

**Spans survive a new `SourceMap` because ids are pinned.** `SourceId`s are indices into
`SourceMap::files` (`crates/nvs-diagnostics/src/source.rs:283`). The session holds a `Pins` table,
canonical path to `SourceId`, and `SourceMap::pinned(&mut Pins)` makes `load` give a pinned path its
pinned id and a new path the next free id, which it adds to `Pins`. A pinned id that this analysis does
not load holds an empty placeholder file. That costs one `SourceFile` per project file per analysis.
So every cached span, `ExprTypeTable` key and row stays valid as it is. The fallback, if pinning breaks
a reader of `SourceMap`: a `FileOutput` stores its file's path, and the fold rewrites its `SourceId`s,
one walk per cached output per analysis. `Pins` lives in `nvs-diagnostics` beside `SourceMap`.

**3. One `TypeInterner` per server session.** The server holds one `TypeInterner`
(`crates/nvs-types/src/ty.rs:507`) for as long as it runs, so a cached `TypeId` stays valid. It is
spent in the editor process, never on the request path. It grows with the distinct types the project
and its edits produce, and an edit that writes new literal types adds to it. So the session rebuilds it
when its size reaches twice its size after the last time `pending` emptied. The rebuild drops the
interner and every `FileOutput`, and the idle turns settle the tree again. The cost is one full
settle, far apart. Its memory goes in the doc comment of the field and the rule fragment.

**4. Each file has a declaration-surface hash.** `crates/nvs-lsp/src/surface.rs` hashes what other
files can see of a parsed file: its namespace and `use` imports, every class, interface, trait and
enum with its header, attributes and member signatures (types, defaults, constant values, property
hooks' signatures, enum cases), every function's signature, every file-scope constant, and every
`require` and `autoload`. It leaves out every function and method body, every span and every other
statement at file scope. Where it is unclear whether something is visible, it goes in the hash. A hash
that includes too much costs time, and one that misses something gives a wrong answer. A body-only
edit keeps the hash. `refresh` then re-indexes the changed file through its open analysis and queues
no reader. An edit that changes the hash queues every entry whose `reads` include the file, as
`refresh` does now. `Indexed` stores the hash.

**5. A cached output is valid for one key.** The editor puts a file's bodies in `bodies` when its
`FileOutput` key differs. The key is the file's content hash, the hash of the surfaces of every
other file in the graph, the hash of the extension set and the configuration inputs `analyse_over`
passes, and the starting `anon_fn_seq`. A finer key, the surfaces the file's bodies actually read, is
not built: a surface edit is much rarer than a body edit, and the test oracle below is simpler to
trust for one combined hash. The oracle is `a_cached_analysis_reports_what_a_fresh_one_does_after_every_kind_of_edit`.
It edits every kind the surface rule names: a body, a signature, a constant value, an attribute, a
route, a closure added at file scope, a class renamed, a file added and a file deleted. After each
edit it compares the diagnostics and every `Indexed` field of the cached analysis with a fresh one that
has no cache. The fresh analysis is today's path, which `nvs check` agrees with.

**6. The index is kept on disk.** `crates/nvs-lsp/src/disk_index.rs` writes every `Indexed` field
(declarations, occurrences, imports, requests, unfollowed, queries, sole kind, `errors`, `broken`,
owning entry, surface hash) and `SymbolIndex::reads`, in a binary format written by hand over the
index's own types. No new dependency (`rule:ide/dependencies-are-allowlisted`). The layout is a magic
`NVSIDX`, a `u32` format version, the 32-byte build stamp, the canonical root, then the entries. Each
entry carries the file's size, modification time in nanoseconds, content hash and surface hash, and
the file ends with a BLAKE3 checksum of everything before it.

- **Where:** `nvs_config::data::Folder::lsp_index()` returns `cache/lsp-index/`
  (`crates/nvs-config/src/data.rs:149`). The file name is the first 16 hex digits of the BLAKE3 hash of
  the canonical workspace root, with `.idx`. A client that names no workspace root gets no on-disk
  index. The stage checks that the compiled-code cache never deletes files in `lsp-index/`. If it
  does, the directory is `lsp/index/` (`crates/nvs-config/src/data.rs:177`).
- **The build stamp** is `nvs_config::cache::build_stamp` (`crates/nvs-config/src/cache.rs:271`),
  made `pub`. A build with no stamp (`build_is_identified` is false) reads and writes no index file, as
  the compiled-code cache does.
- **Load, at start:** the server reads the file once. It discards the whole file for a wrong magic,
  format version, build stamp or checksum. For each entry, a size and modification time that match
  are trusted. Otherwise the server hashes the file's content. If the hash matches, the entry is
  trusted and its time is updated. If not, the server parses that one file for its surface, and its
  entry goes to `pending`. If the surface differs, every entry that read the file goes to `pending`
  too. A file that no longer exists leaves the index, and its readers are queued. A file of the tree
  that is not in the index is queued. So a cold start parses the open document's graph and the files
  that changed, and nothing more.
- **The decoder treats the file as untrusted input.** Each length is checked against the bytes that
  remain before anything is allocated, and every error discards the file. An index file never
  changes what is compiled. It only feeds editor answers, and those are replaced as soon as an entry
  is settled again.
- **Write:** only when the index changed, after `pending` is empty, at most once per idle period. The
  server writes `<name>.<pid>.<n>.tmp` in the same directory and renames it over the index file. It
  does not `fsync`: a torn file fails its checksum and is discarded. A rename that fails, as on
  Windows while another process has the target open, deletes the temporary file, writes nothing and
  tries again at the next idle period. Every way out of a write deletes its temporary file. Two
  servers on one root each write a whole file, and the last rename wins.
- **No background thread** writes or loads (`rule:ide/the-server-is-synchronous`). The write is one
  idle turn's work.

**Memory is spent on purpose, and the doc comments say how much.** Memory is last in the priority
ordering. This goal spends it to buy latency on every keystroke. For every indexed file, the session
keeps its parsed statements, its parse diagnostics, its `FileOutput` and its `Indexed` entry. That is
O(project), with no growth from edits beyond the interner's bound in decision 3. A file's entries are
freed when it leaves the tree or its last owning entry leaves. Stage 5 measures the bytes per file on
`tree(20, 40)` and writes the figure into the doc comments of `ParseCache` and `CheckCache` and into
the rule fragment.

**The rule fragment.** The gate stage writes `docs/rules/ide/the-index-is-incremental-and-persistent.md`
with these decisions and their reasons. Its Declined list:

- A full rebuild at every start.
- A dependency for serialisation.
- A background thread for analysis, load or write (`rule:ide/the-server-is-synchronous`).
- Keeping parsed statements on disk. The format would mirror the whole syntax tree, and the open
  document's graph parses in about 31 ms.
- A per-file dependency set in the cache key, in place of the graph's combined surface hash.
- A stat-based skip of the file read in the parse cache.

It also updates `rule:ide/a-full-reanalysis-stays-under-a-bound` (a full analysis is now the cold and
invalidated path, and a keystroke's cost is the counted one) and
`rule:ide/five-features-are-one-reference-index` where it describes what a refresh re-indexes.

**Lanes.** Stages 1, 2, 4, 5 and 6 share `crates/nvs-lsp/src/document.rs` or `index.rs` and run in
order. Stage 3 touches only `crates/nvs-types/` and `crates/nvs-diagnostics/src/source.rs`, which no
other stage touches. So it may run beside stage 2 or 4. Stage 5 needs stages 2, 3 and 4.

**Red tests.** Stage 1 writes the counted tests of stages 2, 4 and 5 that compile against its own API:
`an_unchanged_file_is_parsed_once_across_keystrokes`, `a_body_only_keystroke_re_queues_no_reader`,
`a_body_only_keystroke_in_a_library_file_parses_one_file_and_checks_one_files_bodies` and
`a_body_only_keystroke_costs_the_same_with_two_programs_and_with_twenty`. They are red until their stage
lands, on this goal's own branch. Each later stage writes its own tests once its API exists.

## Stages

### 1. a generated perf fixture and tests that count the files parsed and the bodies checked

**Does:** `Work` exists and is counted in `analyse_over` and summed by `SymbolIndex`. The generated
fixture and the counted tests exist. The tests that need later stages fail with the counts they found.

- [x] `Work { parsed, checked }` in `Analysed`. Today `parsed` is the files the walk loaded and
  `checked` is `files.len()` passed to `check_program_loaded`. `rule:ide/the-server-is-synchronous`
  `crates/nvs-lsp/src/document.rs:676`, `crates/nvs-lsp/src/document.rs:772`
- [x] `SymbolIndex::work()` and `take_work()` sum the analyses of `refresh`, `settle_one` and
  `absorb`. `crates/nvs-lsp/src/index.rs:484`, `crates/nvs-lsp/src/index.rs:539`,
  `crates/nvs-lsp/src/index.rs:911`
- [x] `crates/nvs-lsp/tests/incremental.rs`: the `TempDir`, `tree(programs, library)` and a
  `keystroke(index, documents, path, edit) -> Work` helper that edits, refreshes and settles until
  `pending` is empty. `crates/nvs-lsp/tests/incremental.rs:1`
- [x] The tests of the `lsp-work-counted` check, and the four red tests the standing decisions name.
  Run once by name: `cargo test --test incremental`. Write the red ones' counts in the handoff.
  `crates/nvs-lsp/tests/incremental.rs:1`

### 2. a file is parsed again only when its text changed: the loader takes a cached parse

**Does:** decision 1. The walk takes a `Parses`, and `Loaded::stmts` is shared. `analyse_over`
passes the session's `ParseCache`, and `Work::parsed` counts its misses.

- [x] The `Parses` trait and the hook at the walk's four `parse_file` sites. `resolve_program_linted`
  and `resolve_program_borrowing` gain a `parses` argument, which `nvs check` passes as `None`.
  `crates/nvs-hir/src/requires.rs:242`, `crates/nvs-hir/src/requires.rs:302`,
  `crates/nvs-hir/src/requires.rs:500`, `crates/nvs-hir/src/requires.rs:597`
  - [x] `Parses::parse` parses and runs `check_declarations`, and `parse_fresh` is the `None` path.
  - [x] The four sites call `parse_fresh`, and every caller passes `None` but the editor.
- [x] `Loaded::stmts` becomes `Arc<Vec<Stmt>>`. List every site that changes a file's statements, and
  use `Arc::make_mut` there. `crates/nvs-hir/src/requires.rs:155`
  - [x] No site changes a file's statements after the walk: the workspace built with only the type
    changed, so no `Arc::make_mut` was needed.
- [x] The hir tests of the `hir-parse-cache` check, in the `requires.rs` test module.
  `crates/nvs-hir/src/requires.rs:1968`
- [x] `ParseCache` in `crates/nvs-lsp/src/parse_cache.rs`, held by the session and passed through
  `analyse_over`. Its doc comment says what it spends per file. Files leave it when they leave the
  tree. `crates/nvs-lsp/src/parse_cache.rs:1`, `crates/nvs-lsp/src/document.rs:721`
  - [x] `Documents::parses` holds it, and `analyse_over` parses the entry through it too.
  - [x] `SymbolIndex::settle_one` calls `Documents::retain_parses` once `pending` empties.
- [x] The lsp tests of the `lsp-parse-cache` check go green. Run once by name:
  `cargo test --test incremental`. `crates/nvs-lsp/tests/incremental.rs:1`

### 3. the checker checks the bodies of a chosen set of files and folds every other file's cached output

**Does:** decision 2 and the pinned ids, in `nvs-types` and `nvs-diagnostics` alone.

**Split:** first, an `Explore` subagent lists every write that the per-file walk makes into a table
shared across files. Its scope is `Env`'s fields at `crates/nvs-types/src/check.rs:225-267` and every
`exprs` write inside `check_stmts` that is not keyed by the current file's span. The answer is
`file.rs:NN` and one line each. The session writes the code itself.

- [x] `Pins` and `SourceMap::pinned`, with placeholder files for ids not loaded. Stage 2 built it,
  because a cached parse's spans are only right where the file keeps its id.
  `crates/nvs-diagnostics/src/source.rs:283`, `crates/nvs-diagnostics/src/source.rs:407`
- [x] `FileOutput` and `CheckCache`, and `ExprTypeTable` split per file and merged in file order.
  `crates/nvs-types/src/expr_table.rs:1605`
  - [x] the Explore list of cross-file writes and reads, checked against the walk
  - [x] `ExprTypeTable::part`/`absorb`, and the row tables' `absorb`
  - [x] `FileOutput` and `CheckCache` in `crates/nvs-types/src/partial.rs`
- [x] `check_program_partial`, with `check_program_loaded` as its whole-program call. The per-file
  walk writes into a `FileOutput`. A file outside `bodies` takes its cached one, or is checked when
  its starting `anon_fn_seq` moved. Every pass after the walk runs over the folded result.
  `crates/nvs-types/src/check.rs:158`, `crates/nvs-types/src/check.rs:224`,
  `crates/nvs-types/src/check.rs:289`
  - [x] the walk writes each file into a fresh part, folded in file order
  - [x] a cached output is folded where its file is outside `bodies` and its start count matches
  - [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` green
- [x] The tests of the `types-partial-check` check, in `crates/nvs-types/tests/partial_check.rs`,
  through the helper at `crates/nvs-types/tests/common/mod.rs:315`. Run once by name:
  `cargo test --test partial_check`. `crates/nvs-types/tests/partial_check.rs:1`
  - [x] a canonical dump of a table, so two tables compare
  - [x] the five tests, run once by name
- [x] The measured cost of the passes after the walk on a 40-class library, in the doc comment.
  `crates/nvs-types/src/check.rs:158`

### 4. each file has a declaration-surface hash, and a body-only edit re-queues no reader

**Does:** decision 4. `surface.rs` hashes a parsed file. `Indexed` stores the hash, and `refresh`
queues readers only when the hash changed.

- [x] `crates/nvs-lsp/src/surface.rs`: the hash over the parse that `ParseCache` holds, with what
  it includes and leaves out written in its module doc. `crates/nvs-lsp/src/surface.rs:1`
- [x] `Indexed` gains `surface`. `refresh` compares the old and new hash of `changed`, and queues
  readers only when they differ. `crates/nvs-lsp/src/index.rs:336`, `crates/nvs-lsp/src/index.rs:484`
- [x] The tests of the `lsp-surface` check. Run once by name: `cargo test --test incremental`.
  `crates/nvs-lsp/tests/incremental.rs:1`

### 5. the editor checks only the bodies whose inputs changed, against one session interner

**Does:** decisions 3 and 5. The server holds one `TypeInterner`, `Pins`, `ParseCache` and
`CheckCache`. `analyse_over` computes each file's key and passes `check_program_partial` the files
whose key changed. The acceptance's keystroke count holds.

- [ ] A `Session` struct holds the interner, `Pins`, `ParseCache` and `CheckCache`. It is passed
  where `analyse_over` is reached, and the server loop owns it. `crates/nvs-lsp/src/document.rs:676`,
  `crates/nvs-lsp/src/server.rs:261`
- [ ] `analyse_over` uses `SourceMap::pinned`, the session interner and `check_program_partial` with
  the key of decision 5. `Work::checked` counts `bodies`. `crates/nvs-lsp/src/document.rs:682`,
  `crates/nvs-lsp/src/document.rs:743`, `crates/nvs-lsp/src/document.rs:772`
- [ ] The interner rebuild at twice its size after the last settle. `crates/nvs-lsp/src/server.rs:300`
- [ ] Measure the bytes per file on `tree(20, 40)`, and write them into the doc comments of
  `ParseCache` and `CheckCache`. `crates/nvs-lsp/src/parse_cache.rs:1`
- [ ] The tests of the `lsp-incremental` check, the oracle included. Run once by name:
  `cargo test --test incremental`. `crates/nvs-lsp/tests/incremental.rs:1`

### 6. the index is written to disk and loaded at start, and only stale entries are analysed

**Does:** decision 6. The server loads the index file at start, queues only the stale entries, and
writes the file once per idle period after `pending` empties.

- [ ] `Folder::lsp_index()`, and `build_stamp` made `pub`. Check that the compiled-code cache leaves
  `lsp-index/` alone. `crates/nvs-config/src/data.rs:149`, `crates/nvs-config/src/cache.rs:271`
- [ ] `crates/nvs-lsp/src/disk_index.rs`: encode and decode over `Indexed` and `reads`, with the
  bounded, checksummed decoder. `crates/nvs-lsp/src/disk_index.rs:1`
- [ ] `SymbolIndex::load_from(documents, scope, root, path)` validates each entry and queues only the
  stale ones. `SymbolIndex::save_to(path)` writes atomically and deletes its temporary file on every
  failure. `crates/nvs-lsp/src/index.rs:453`
- [ ] The server loads the file at start in place of `planned`, and writes it once per idle period
  after `settled`. `crates/nvs-lsp/src/server.rs:261`, `crates/nvs-lsp/src/server.rs:601`
- [ ] The tests of the `lsp-disk-index` check. The second-server test runs two `save_to` calls on
  one path from two threads, waits on both and reads one whole file. It never waits on time. A
  change to paths runs its tests in WSL once with `bun nv linux-test`. Run once by name:
  `cargo test --test incremental`. `crates/nvs-lsp/tests/incremental.rs:1`

### 7. the goal is verified once, and every failure is fixed

- [ ] Write `docs/rules/ide/the-index-is-incremental-and-persistent.md` with the standing decisions,
  the memory figures and the Declined list. Update `rule:ide/a-full-reanalysis-stays-under-a-bound`
  and `rule:ide/five-features-are-one-reference-index`. `bun nv rules --check` is green.
  `docs/rules/ide/a-full-reanalysis-stays-under-a-bound.md:1`,
  `docs/rules/ide/five-features-are-one-reference-index.md:1`
- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.
- [ ] The goal-end gates: `bun nv verify --doc`, `bun nv owners --closes incremental-index` and
  `bun nv ci --check`, each red fixed.

## Handoff

### State

Stages 1 to 4 are done. `nvs_lsp::surface::of` (`crates/nvs-lsp/src/surface.rs`) hashes each
declaration's text with its bodies cut out. `Indexed::surface` holds it, and `refresh`
(`crates/nvs-lsp/src/index.rs:499`) queues an open changed file's readers only when its new hash
differs. A changed file nobody has open still queues every reader: stage 6 may parse it for its
surface instead. `surface::of_source` parses a text alone, for tests and stage 6.
`check_program_partial` (`crates/nvs-types/src/check.rs:209`) and `CheckCache`
(`crates/nvs-types/src/partial.rs:118`) are stage 3's; a one-body partial check takes about 0.2 ms
against 0.7 ms whole on 41 files.

The sweep's reds were stale: no `env-secret-proofs` check exists, and nightly `41428598621` was
cancelled by a later push. The untracked `probe-out.txt` at the root is another process's.

**Red until stage 5:** `a_body_only_keystroke_in_a_library_file_parses_one_file_and_checks_one_files_bodies`
(checked is 42, the open file's whole graph), so `bun nv verify --fast -p nvs-lsp` is red. The other
nine `cargo test --test incremental` tests ran once, green. The rest of the suite did not run.

### Next

**Stage 5, one group.** `crates/nvs-lsp/src/document.rs` and `server.rs`.

- [ ] A `Session` with the interner, `Pins`, `ParseCache` and `CheckCache`, owned by the server loop. `crates/nvs-lsp/src/document.rs:690`
- [ ] `analyse_over` calls `check_program_partial` in place of `check_program_loaded`, keyed by decision 5 with `surface::of`; `Work::checked` counts `bodies`. `crates/nvs-lsp/src/document.rs:817`
- [ ] A file's output is removed when it leaves the tree, beside `retain_parses` in `settle_one`. `crates/nvs-lsp/src/index.rs:554`
- [ ] The interner rebuild at twice its size, the bytes-per-file figure, and the `lsp-incremental` tests with the oracle. `crates/nvs-lsp/tests/incremental.rs:1`

### Backlog

Carried from `release-run`, which ended with `v0.0.1` published. None of these is this goal's work.

- `bun nv ci --publish-release` prints the draft's `html_url`, an `untagged-…` address, after the
  publish. It should print `releases/tag/<tag>`. `tools/nv/cmd/ci.ts:1071`
- `--watch --release` should pass over a failed optional `build` leg the way `--release-status`
  does, reading the legs from `build.yml`'s matrix, with a test in `tools/nv/test/ci.test.ts`.
- `cargo clippy` on macOS warns `chunks_exact` with a constant chunk size at
  `crates/nvs-runtime/src/child_watch.rs:845`. CI's `lint` job runs on Linux, so the macOS-only code
  is linted nowhere.
- `nvs info --licenses | head` panics with "failed printing to stdout" once the pipe closes. A CLI
  command that prints should end quietly on a closed stdout.
- `crates/nvs-server/src/serve.rs` is 14,400 lines, and `nvs-stdlib` is the largest crate: a split waits for a measured cost (`docs/agent/coding.md`).
- The website builds twice per push, in `.github/workflows/pages.yml` and in `ci.yml`'s `proofs` job (`rule:testing/ci-lanes`).
- `tools/nv/cmd/ci-changes.ts`'s `proofs` filter has no `extensions/`, so `bun nv pdf-validate` may not run on a change only to a component.
- `main` has the user's unpushed code commits `2dfacf609` and `d4e4e79a4`. They go out with the next `bun nv ci --push`.
