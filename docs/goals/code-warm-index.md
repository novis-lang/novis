---
looper: 1
title: "nvs code answers from the editor's index, and calls and reach take several methods"
summary: "Reached when a cold index keeps every worker busy and checks each body once, nvs code loads and saves the editor's index file, checks every entry and every input against its content hash, gives the answer a cold build gives after every kind of edit, settles the stale entries on worker threads, takes several methods in one calls or reach call, and the two rules, the help, about.md and the website say so."
context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/workers.rs, docs/rules/tooling/nvs-code-answers-about-the-project.md, docs/rules/ide/the-index-is-incremental-and-persistent.md]
stages:
  - id: 1
    title: the two rules say that nvs code shares the editor's index, and how it is never stale
    status: done
  - id: 2
    title: a cold index keeps every worker busy and checks each body once
    status: done
    context: [crates/nvs-lsp/src/document.rs, crates/nvs-lsp/tests/incremental.rs]
  - id: 3
    title: nvs code loads, checks by content and saves the editor's index file
    status: done
    context: [crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/disk_index.rs, crates/nvs-cli/src/code.rs, crates/nvs-lsp/tests/index.rs]
  - id: 4
    title: calls and reach take several methods
    status: done
    context: [crates/nvs-cli/src/code.rs, crates/nvs-cli/tests/code.rs]
  - id: 5
    title: the help, the reference, about.md and the website
    status: done
    context: [docs/reference/tools/10-cli.md, docs/examples/tools/cli/nvs-code/about.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: rules
    stage: 1
    name: "the rulebook validates with the two rewritten rules"
    run: [bun, nv, rules, --check]
    timeout: 900
  - id: parallel
    stage: 2
    name: "a cold index of many programs plans on every worker at once and checks each body once"
    run: [cargo, test, --test, incremental, --, a_cold_index_of_many_programs_plans_on_every_worker, a_cold_index_checks_each_body_once]
    timeout: 1800
    tests:
      - a_cold_index_of_many_programs_plans_on_every_worker
      - a_cold_index_checks_each_body_once
  - id: never-stale
    stage: 3
    name: "a loaded index gives the answer a cold build gives after every kind of edit, and an unchanged tree analyses nothing"
    run: [cargo, test, --test, index, --, a_content_checked_load_answers_as_a_cold_build_after_every_kind_of_edit, a_changed_file_with_its_old_size_and_time_is_analysed_again, a_changed_configuration_input_is_analysed_again, a_content_checked_load_of_an_unchanged_tree_analyses_nothing]
    timeout: 1800
    tests:
      - a_content_checked_load_answers_as_a_cold_build_after_every_kind_of_edit
      - a_changed_file_with_its_old_size_and_time_is_analysed_again
      - a_changed_configuration_input_is_analysed_again
      - a_content_checked_load_of_an_unchanged_tree_analyses_nothing
  - id: code-index
    stage: 3
    name: "nvs code saves the index in its data folder, and an edit between two calls reaches the second answer"
    run: [cargo, test, --test, code, --, the_first_call_saves_the_index_in_the_data_folder, an_edit_between_two_calls_reaches_the_second_answer, a_call_without_a_data_folder_answers_cold]
    timeout: 1800
    tests:
      - the_first_call_saves_the_index_in_the_data_folder
      - an_edit_between_two_calls_reaches_the_second_answer
      - a_call_without_a_data_folder_answers_cold
  - id: several
    stage: 4
    name: "calls and reach take several methods, keep the one-name output, and fail on any unknown name"
    run: [cargo, test, --test, code, --, reach_of_several_methods_prints_each_under_its_name, calls_of_several_methods_prints_each_under_its_name, several_methods_in_json_are_a_methods_list, one_unknown_method_among_several_fails_the_call_and_names_it, one_method_prints_what_it_printed_before]
    timeout: 1800
    tests:
      - reach_of_several_methods_prints_each_under_its_name
      - calls_of_several_methods_prints_each_under_its_name
      - several_methods_in_json_are_a_methods_list
      - one_unknown_method_among_several_fails_the_call_and_names_it
      - one_method_prints_what_it_printed_before
  - id: proofs
    stage: 5
    name: "nvs code has every feature proof"
    run: [bun, nv, proofs, --id, "tools:cli/nvs-code", --gate]
    timeout: 1800
  - id: site-stale
    stage: 5
    name: "every page this goal reaches is reread and stamped"
    run: [bun, nv, site, --check, stale]
    timeout: 900
  - id: site-changes
    stage: 5
    name: "nvs code's faster answers and several methods have their changes.json entry"
    run: [bun, nv, site, --check, changes]
    timeout: 900
  - id: site-review
    stage: 6
    name: "every user page this goal changed carries its newcomer review stamp"
    run: [bun, nv, site, --check, review]
    timeout: 900
---

# nvs code answers from the editor's index, and calls and reach take several methods

## Why

Every `nvs code` call analyses the whole project on one thread. On this repository, 9,105 `.nvs`
files, one call takes 27 s (`nvs code find Cart`, release build). A tool that asks about each
changed method pays that once per method.

- `crates/nvs-cli/src/code.rs:312` calls `nvs_lsp::workspace` (`crates/nvs-lsp/src/server.rs:2533`),
  which runs `SymbolIndex::build` with no workers (`crates/nvs-lsp/src/index.rs:833`) and never loads
  the editor's index file (`SymbolIndex::load_from`, `crates/nvs-lsp/src/index.rs:534`).
- The editor's warm start from that file takes 1.07 s on the same tree (`bun nv lsp-start .`).
- The editor's cold start is parallel and still slow: 18.7 s on 15 workers, and 31.9 s while the loop
  was building. Sampled once a second, the process used about 2.2 cores on average out of 16, and
  its CPU time was about twice the serial call's. The count was `parsed=13698 checked=17402` for
  9,105 files, so bodies were checked about twice.
- The likely serial point, **not proven**: the look-ahead at `crates/nvs-lsp/src/index.rs:956`
  stops at the first entry being planned, so only one plan runs at a time. This tree has thousands
  of one-file programs, and for those the plan is most of the work.

The *"A disk cache of the index"* decline in `rule:tooling/nvs-code-answers-about-the-project` was
written before the editor had an index file, when a cache meant a second, unchecked copy. The
editor's file, with its per-file checks, came later. The user decided that `nvs code` shares it,
provided it is never stale.

## Standing decisions

The user decided: `nvs code` shares the editor's index file; it must never give a stale answer;
`calls` and `reach` both take several methods; the cold index is made parallel in this goal; the
goal is last on the chain. The rest are the authoring session's calls, made by the priority
ordering. A session builds on them and does not reopen them.

**Never stale, by five guarantees.** A loaded index is trusted only where each holds:

1. **Content, not time.** `nvs code` compares every indexed file's BLAKE3 content hash with the
   file's bytes. It never trusts a matching size and modification time
   (`crates/nvs-lsp/src/index.rs:1889`). Reading and hashing every file of this repository took
   0.7 s on one thread in Bun; the hash pass runs on the workers. The editor keeps its stamp check,
   since a stale editor answer lasts one keystroke.
2. **The file list is walked fresh.** A new file is analysed. A gone or newly ignored file leaves the
   index, and every entry that read it is analysed again. This is `load_from_tree` as it is.
3. **A surface change re-analyses every reader**, as the editor already does.
4. **Another build discards the whole file**, through the build stamp.
5. **Every input outside the `.nvs` files is checked too**: each `nvs.toml` and settings file an
   analysis read, each `.nvsx` extension, and each ignore file. Whether the index file records these
   today is **not checked**. Stage 3's first item finds out. Each input it does not record is added
   to the format with its content hash, under a new format version. Where that is not possible, a
   change to it discards the whole file.

**The proof is a comparison with a cold build.** A test edits a generated tree in every way: a body,
a signature, adding, deleting and moving a file, an ignore file, `nvs.toml`, an extension, and new
content with the old size and time. After each edit, the content-checked load must give the index a
cold `SymbolIndex::build` gives, compared as the editor's oracle test compares
(`a_cached_analysis_reports_what_a_fresh_one_does_after_every_kind_of_edit`,
`crates/nvs-lsp/tests/incremental.rs:575`). **Safe default:** any input the test cannot prove makes
`nvs code` run cold. A slow answer is acceptable and a stale one is not.

**Who reads the file.** The editor and `nvs code`. `nvs check`, compiling and running never read it,
because they are the authoritative paths and the file is untrusted input. The decoder stays as it is:
magic, version, build stamp, checksum, and each length checked before it allocates.

**Who writes it.** `nvs code` saves through `save_to` (`crates/nvs-lsp/src/index.rs:644`) into the
same path the editor uses (`index_file`, `crates/nvs-lsp/src/server.rs:737`). It saves only when it
analysed anything. Two writers each rename a whole file, and the last rename wins, as the rule
already says for two servers. A failed save prints nothing and changes no answer. A call without a
data folder or without a build stamp runs cold, on the workers.

**The workers.** `nvs code` settles on `use_workers(0)` (`crates/nvs-lsp/src/index.rs:847`), cores
minus one, stopped before the answer prints. Tests pass `--data <scratch>` so no index file reaches
the binary's own data folder.

**Parallel cold index.** Stage 2 first measures by counts which files are checked more than once,
and why, then fixes the cause: the plan of an entry ahead of its turn runs while earlier entries
plan, and each body is checked once per graph. The merge stays in entry path order on the server
thread, so the parallel index equals the serial one (`crates/nvs-lsp/src/workers.rs:15`). No worker
cap and no setting. Measured with `bun nv lsp-start . --cold`, never timed in a test: the tests count
plans in flight and bodies checked.

**Several methods.** `calls` and `reach` take `<METHOD>...`. With one name, the output is byte for
byte what it is today. With several, each name, in the order given and printed once, gets a line with
its name and then its own lines indented by two spaces. `--json` with several prints
`schemaVersion` and `methods`, a list of the one-name documents without their `schemaVersion`. Any
unknown name fails the whole call with exit status 1 before anything is printed. The text names each
unknown name with its nearest names, and the JSON gives `error` and `unknown`, a list of `symbol` and
`nearest`. One unknown name alone keeps today's output. `show`, `refs` and `impls` keep one name.

**What it costs.** No request path is touched. Memory per call is unchanged at its peak, and a
loaded index holds less than a cold one while it settles. Disk: one index file per project root, as
the editor already writes, so it grows with projects, never with calls.

**Lanes.** Stages 2 and 3 share `crates/nvs-lsp/src/index.rs`, and stage 4 builds on stage 3's
`code.rs`, so the stages run in order. One goal, no side lanes.

**What the index file records outside the `.nvs` files.** Stage 3's first item found that format 1
recorded nothing: no `nvs.toml`, settings file or `.nvsx`. Format 2 records every configuration input
of the entries' directories (`crate::tree_cache::inputs`): the `nvs.toml` beside each, the data
folder's file where that one is absent, each file and directory the tree read or looked for, and each
`.nvsx` the tree's `[[extension]]` names. Each is kept with its content: absent, the BLAKE3 of a file's
bytes, the BLAKE3 of a directory's sorted names, or unreadable. A load recomputes the inputs of the
saved entries' directories, and **any one changed or not recorded discards the whole file**. Per-entry
invalidation was declined: one `nvs.toml` is read by every entry of its directory, so it would rerun
the same entries with more code. The inputs are taken when the index is planned or loaded, so an input
edited while the index settles is recorded as it was and discards the file at the next load. An ignore
file is not an input: the walk is fresh, and an entry point the walk no longer selects leaves the
loaded index with the files it indexed, and its readers wait.

**What stops it.** One attempt per item and two sessions per stage. A stage that does not close in
two sessions writes what holds it in the handoff, and the next session splits it.

## Stages

### 1. the two rules say that nvs code shares the editor's index, and how it is never stale

- [x] `docs/rules/tooling/nvs-code-answers-about-the-project.md:88`: "A call costs one cold
  analysis" becomes the load, the five guarantees, the workers and what it costs. `calls` and `reach`
  take several methods. The *disk cache* decline is removed. The *background daemon* decline stays.
  A new decline: trusting size and time in `nvs code`, because a stale answer outlives a keystroke
  there.
- [x] `docs/rules/ide/the-index-is-incremental-and-persistent.md:62`: "only an editor answer reads
  it" names `nvs code` as the second reader, and says `nvs check`, compiling and running never read
  it.
- [x] `bun nv rules --render`, then the `rules` check.

### 2. a cold index keeps every worker busy and checks each body once

- [x] `crates/nvs-lsp/tests/incremental.rs:82`: count, on a generated tree of many one-file programs
  and on this repository with `bun nv lsp-start . --cold`, which files are checked more than once
  and why. Write the cause in the handoff.
- [x] `crates/nvs-lsp/src/index.rs:956`: entries ahead of their turn plan in parallel, up to the
  workers' room. The merge stays in path order.
- [x] `crates/nvs-lsp/src/workers.rs:17`: each body is checked once, by the fix the first item
  names.
- [x] `crates/nvs-lsp/src/document.rs:778`: a `Work` count of the most plans in flight, and
  `crates/nvs-lsp/tests/incremental.rs`: the two `parallel` tests.
- [x] `crates/nvs-lsp/src/workers.rs:47`: § *What it costs* gets the new `bun nv lsp-start . --cold`
  figures: wall time, CPU time and the work counts.

### 3. nvs code loads, checks by content and saves the editor's index file

- [x] `crates/nvs-lsp/src/disk_index.rs:1`: find which configuration inputs an entry records, and
  add each missing one with its content hash under a new format version (guarantee 5).
- [x] `crates/nvs-lsp/src/index.rs:1882`: a content-only `freshness` for `nvs code`, run on the
  workers.
- [x] `crates/nvs-lsp/src/server.rs:2533`: `workspace` loads from the data folder's index file,
  settles on the workers, saves, and stops the workers.
- [x] `crates/nvs-lsp/tests/index.rs`: the four `never-stale` tests.
- [x] `crates/nvs-cli/src/code.rs:305`: `Project::open` takes the loaded workspace, and
  `crates/nvs-cli/tests/code.rs`: the three `code-index` tests, each with `--data` in its scratch.

### 4. calls and reach take several methods

- [x] `crates/nvs-cli/src/code.rs:232` and `crates/nvs-cli/src/code.rs:247`: `Calls` and `Reach`
  take one or more names, and their help says so (AGENTS.md § *Text an end user reads*).
- [x] `crates/nvs-cli/src/code.rs:843` and `crates/nvs-cli/src/code.rs:922`: one load, then each
  name, in the shapes § *Standing decisions* gives, and every unknown name judged first.
- [x] `crates/nvs-cli/tests/code.rs`: the five `several` tests.

### 5. the help, the reference, about.md and the website

- [x] `docs/reference/tools/10-cli.md:1083`: `# nvs code` says that a call reuses the editor's
  index, that it checks every file's content first, and that `calls` and `reach` take several
  methods.
- [x] `docs/examples/tools/cli/nvs-code/about.md:1`: the same, held to AGENTS.md § *Text an end user
  reads*. `docs/examples/tools/cli/nvs-code/01-who-calls-a-method.nvs:1` shows two methods in one
  call.
- [x] `website/src/content/docs/tools/editor/index.mdx:210`: one sentence that the editor and
  `nvs code` share one index.
- [x] `website/src/data/changes.json`: `changed` `nvs code`, covering the faster answers and several
  methods in `calls` and `reach`.
- [x] Reread and stamp every page `bun nv site --check stale` names, then the `proofs` check.

### 6. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of every page this goal changed ([prose.md](../agent/prose.md) § *A
  newcomer reads it*).
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 5 are done and committed. The `never-stale`, `code-index` and `several` tests pass,
each run once by name. `site-stale` and `site-changes` are green.

Stage 5 (`e211c8832d`): the reference's `# nvs code` gained `## The index file`, `## The verbs`
and the several-methods shape; the help, `about.md`, the example (two methods in one `calls`,
checked against a real run), the editor page and `changes.json` say the same.
`proofs` is red for `tools:cli/nvs-code` only on the `about.md` review, which stage 6 does. Its
`--gate` covers the whole roster, where 146 features outside this goal also owe proofs.

- `crates/nvs-cli/src/code.rs`: `Project::open` loads and saves `index_file_of(&root)`.
  `per_method` drives `calls` and `reach` in the shapes § *Standing decisions* gives, and
  `unknown_methods` fails several names first, with `nearest to <name>:` for each.
- `crates/nvs-cli/tests/code.rs` `run` passes `--data <cwd>/.nvsdata` on every call.
- `crates/nvs-lsp`: `load_by_content`, `freshen`, format 2's `Input`s, `workspace_from`.

Only the new tests, the build and clippy ran. Watch at the gate: the disk tests in
`crates/nvs-lsp/tests/incremental.rs` from `a_cold_start_with_a_valid_index_...` to
`two_servers_writing_one_root_...`, the three `parallel` ones, `tests/query.rs`, and every test
in `crates/nvs-cli/tests/code.rs`, which now load the index file and run through `per_method`.

The nightly `miri units (nvs-stdlib 2/12)` failure was a 300 s timeout of
`granularity::tests::both_walks_agree_on_every_window` on commit `7e97fa8`. `156f11a7ca` lowered
its Miri reach to 2 after that run, so the fix is already in the tree. The next nightly confirms it.

### Next

Stage 6, a close (not the full gate). First the newcomer review (prose.md § *A newcomer reads
it*) of the five texts `bun nv site --check review` names:
`docs/examples/tools/cli/nvs-code/01-who-calls-a-method.nvs`, its `about.md`,
`help:tools:cli/nvs-code`, `website/src/content/docs/tools/editor/index.mdx` and
`website/src/data/changes.json`. Then the build, clippy and this goal's tests by name: the
`parallel`, `never-stale`, `code-index` and `several` checks. Expected red until done:
`site-review`, and `proofs` for the review of `about.md`.

### Backlog

- On `bun nv lsp-start --copies 12 --cold --edits 5`, five surface edits check 5833 bodies, about
  1166 an edit over 972 files. The commit before stage 2 gives the same count, so it is older than
  this goal. Not in this goal's scope; a later goal finds which entry checks a file twice on an edit.
- `cargo fmt --all` on this machine reformats seven files nobody edited (`crates/nvs-cli/src/runner.rs`,
  `crates/nvs-stdlib/src/http.rs` and others). They were reverted. `bun nv ci --check` at the next full
  gate says whether the tree or the local rustfmt is behind.
