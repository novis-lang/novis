---
looper: 1
title: a finished run's records are deleted oldest first, and its page says which were deleted
summary: "Reached when a run's rows are deleted by their own time under [debug] index_age and index_max, as a running command's rows already are, a run row goes only once its end is past them, the viewer's run page says which of its records were deleted, and the rules, the docs and the website say so."
context: [crates/nvs-cli/src/debug/index.rs, crates/nvs-cli/tests/debug_ingest.rs, docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md, docs/rules/testing/a-run-is-recorded-like-a-request.md]
stages:
  - id: 1
    title: "a run's rows go by their own time, a run row by its end, and the index keeps the cut"
    status: done
    context: [crates/nvs-cli/src/debug/runs.rs, crates/nvs-cli/tests/debug_api.rs]
  - id: 2
    title: "the run page says which records were deleted, and the rules, the docs and the website say so"
    status: done
    context: [viewer/src/views/run.tsx, viewer/src/api.ts, viewer/test/runs.browser.test.ts, website/src/content/docs/tools/debug-viewer/index.mdx]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: rows
    stage: 1
    name: "a finished run's rows go oldest first by their own time under index_age and index_max, its run row stays until its end is past them, and the API returns the cut"
    run: [cargo, test, --test, debug_ingest, --test, debug_api]
    tests: [index_age_deletes_a_finished_runs_old_rows_and_keeps_its_new_ones, index_max_deletes_a_long_runs_oldest_rows_before_a_newer_request, a_run_row_goes_once_its_end_is_past_index_age, a_run_that_started_before_the_cut_returns_it]
    timeout: 1800
  - id: viewer
    stage: 2
    name: "the viewer's types, its committed build and its unit tests are green"
    run: [bun, nv, viewer, --verify]
    timeout: 900
  - id: close
    stage: 3
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, long-run-records]
    timeout: 900
---

# a finished run's records are deleted oldest first, and its page says which were deleted

## Why

`[debug] index_max` bounds every row in one time order (commit `50a36f29d6`,
`crates/nvs-cli/src/debug/index.rs`, `Index::forget`). A command that is still running has no
`runs` row yet, because its `run` line is written when it ends (`crates/nvs-cli/src/main.rs:2740`).
So its rows have no owner, and they are already deleted one by one, oldest first, under both
bounds.

Once the run ends, its `runs` row owns every row of its trace (`OWNERS` in `index.rs`). Then they
are deleted together, by the run's start: past `index_age` all of them go at once, including the
newest, and past `index_max` the run's start is old, so the whole run goes first. A command that ran
for days loses its last hour of records in the same step as its first. A request is short, so
deleting it whole is right. A run can be as long as the host's uptime, so it is not.

One long trace cannot outgrow the index any more. This goal makes the order the same after the run
ends, and shows on the run page which records are gone.

## Standing decisions

**A run's rows are loose rows.** The rows of a run's trace are deleted by their own `at_us`, in the
same steps that delete rows no owner names. Only a request still deletes its rows with it. In
`index.rs`, `unowned(table)` tests only `requests`. `OWNERS` is a list of `Owner`s, each a table,
the time it is deleted by and the tables deleted with it: a request by `start_us` with every
`BY_TRACE` table, and a run by `start_us + duration_us` with `exceptions` alone. One
`forget_batch` and one `oldest` loop then serve both.

**A run row goes by its end.** A `runs` row is deleted when `start_us + duration_us` is before the
`index_age` cutoff, and under `index_max` it is one more head in `Index::oldest`, timed by its end.
Its end is not before any of its rows, so in the time order its rows go first and the row last. It
takes its `exceptions` row with it, as now. The index `runs_start` stays for the list. A new index
`runs_end` on the expression `start_us + duration_us` serves the two lookups, one entry per run
inside the cap. The schema goes up once.

**The index keeps one cut, not one per run.** `meta` gets a `cut_us` row: the newest `at_us` of any
row a time step deleted. Each `forget_rows` batch raises it to that batch's newest deleted time.
`/api/run` returns `cut_us` when the run started before it, and `null` otherwise. The run page then
says that records before that time were deleted. That is true for every run that started before the
cut, whether or not it had rows that old. A count per run would need an `UPDATE` of every touched run
in every batch, for a note that says the same thing. The user chose the one cut.

**A command that is still running stays out of the Runs view.** It has no `run` line until it ends.
Showing it would need a line written at the start and a second at the end, which is a change to
`rule:testing/a-run-is-recorded-like-a-request` beyond this goal.

**Trade-offs.** No change on the request path or in any server. In `nvs debug`, one more lookup per
deletion step and one `meta` write per batch. On disk, one `runs_end` entry per run, inside the cap.
Usability: a long run keeps its newest records, and its page says which are gone.

**Website.** One `website/src/data/changes.json` entry of kind `fixed` for
`directive:debug.index_max` and `directive:debug.index_age`: a finished run's oldest records are
deleted first, and its page says which records were deleted.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is written to `### Backlog` with what blocked it.

**Lanes.** It is a chain goal, placed right behind `mail-complete` by the user's call. It depends on
no chain goal, and it shares `crates/nvs-cli/src/debug/index.rs` only with a goal that adds a debug
table.

## Stages

### 1. a run's rows go by their own time, a run row by its end, and the index keeps the cut

- [x] `crates/nvs-cli/src/debug/index.rs`: `runs` leaves `OWNERS`, `unowned` tests only `requests`,
  and § *A run row goes by its end* is built: the age pass, the `Index::oldest` head, the
  `runs_end` index and the schema raised once. The module doc's *The deletion window* says so.
- [x] `crates/nvs-cli/src/debug/index.rs`: `forget_rows` raises `meta`'s `cut_us`, and `rebuild`
  starts a fresh file without one.
- [x] `crates/nvs-cli/src/debug/runs.rs:135`: `/api/run` returns `cut_us` as § *The index keeps one
  cut* says.
- [x] `crates/nvs-cli/tests/debug_ingest.rs`: three tests beside
  `index_max_deletes_the_oldest_rows_whether_a_request_names_them_or_not`, built the same way with a
  `run` line from the sample: `index_age_deletes_a_finished_runs_old_rows_and_keeps_its_new_ones`,
  `index_max_deletes_a_long_runs_oldest_rows_before_a_newer_request` and
  `a_run_row_goes_once_its_end_is_past_index_age`.
- [x] `crates/nvs-cli/tests/debug_api.rs:1388`: `a_run_that_started_before_the_cut_returns_it`, and a
  run that started after it returns `null`. Then the check `rows` once.

### 2. the run page says which records were deleted, and the rules, the docs and the website say so

- [x] `viewer/src/api.ts` and `viewer/src/views/run.tsx:44`: the run page shows one note under its
  header when `cut_us` is set: "Records before <time> were deleted." A case in
  `viewer/test/runs.browser.test.ts`. Then `bun nv viewer` to rebuild `viewer/dist/`, and the check
  `viewer` once.
- [x] `docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md`: the
  paragraph *A run is forgotten as a request is* says what § *Standing decisions* says, and its
  Declined list gets *a count of deleted rows per run*. `bun nv rules --render`.
- [x] `docs/rules/testing/a-run-is-recorded-like-a-request.md:28`: *The Runs view* says the page
  shows the cut.
- [x] `docs/examples/config/debug-index_max/about.md` and `docs/examples/config/debug-index_age/about.md`:
  a finished run's records are deleted one by one too. `crates/nvs-config/src/default.toml`'s
  `index_age` and `index_max` comments say the same.
- [x] `website/src/content/docs/tools/debug-viewer/index.mdx`: one sentence where the page describes
  a run. `website/src/data/changes.json:1`: the entry of § *Website*, written to AGENTS.md § *Text an
  end user reads*. Each page reread and stamped with `bun nv site --stamp`.

### 3. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written on 2026-11-26 by the loop session that built stage 2 (`37b67b7c4c`, `8d8adb6e1f`,
`2e6f00a566`). Stages 1 and 2 are done, and the checks `rows` and `viewer` are green. The run page
shows `Cut` (`viewer/src/views/run.tsx`, class `run-cut`) when `cut_us` is set, and its browser
case in `viewer/test/runs.browser.test.ts` passed once by name. Both rules, the two `about.md`,
`default.toml`'s two comments, the viewer page and one `changes.json` entry say what
§ *Standing decisions* says. Every text was stamped and had its newcomer review, and
`site --check stale changes review` names only `Core\Arr` texts. Those belong to the uncommitted
`Core\Arr` work in the tree (`arr.rs`, `core_members.rs`, `docs/rules/types/arrays.md`,
`taint-propagation.md`), which is not this goal's and was not staged.

The acceptance sweep's nine nightly failures are all run 45813627231, which the Backlog below
already accounts for: eight were fixed after the commit it tested, and the ninth is the aarch64
stack-page test.

### Next

Stage 3: close the goal with a build, clippy, the tests this goal wrote by name (the check `rows`
and the new browser case), and `bun nv close --goal long-run-records`. If the `Core\Arr` work is
still uncommitted then, its red `site` lines are not this goal's. No test is expected to go red.

### Backlog` with what blocked it.

**Lanes.** It is a chain goal, placed right behind `mail-complete` by the user's call. It depends on
no chain goal, and it shares `crates/nvs-cli/src/debug/index.rs` only with a goal that adds a debug
table.

## Stages

### 1. a run's rows go by their own time, a run row by its end, and the index keeps the cut

- [x] `crates/nvs-cli/src/debug/index.rs`: `runs` leaves `OWNERS`, `unowned` tests only `requests`,
  and § *A run row goes by its end* is built: the age pass, the `Index::oldest` head, the
  `runs_end` index and the schema raised once. The module doc's *The deletion window* says so.
- [x] `crates/nvs-cli/src/debug/index.rs`: `forget_rows` raises `meta`'s `cut_us`, and `rebuild`
  starts a fresh file without one.
- [x] `crates/nvs-cli/src/debug/runs.rs:135`: `/api/run` returns `cut_us` as § *The index keeps one
  cut* says.
- [x] `crates/nvs-cli/tests/debug_ingest.rs`: three tests beside
  `index_max_deletes_the_oldest_rows_whether_a_request_names_them_or_not`, built the same way with a
  `run` line from the sample: `index_age_deletes_a_finished_runs_old_rows_and_keeps_its_new_ones`,
  `index_max_deletes_a_long_runs_oldest_rows_before_a_newer_request` and
  `a_run_row_goes_once_its_end_is_past_index_age`.
- [x] `crates/nvs-cli/tests/debug_api.rs:1388`: `a_run_that_started_before_the_cut_returns_it`, and a
  run that started after it returns `null`. Then the check `rows` once.

### 2. the run page says which records were deleted, and the rules, the docs and the website say so

- [x] `viewer/src/api.ts` and `viewer/src/views/run.tsx:44`: the run page shows one note under its
  header when `cut_us` is set: "Records before <time> were deleted." A case in
  `viewer/test/runs.browser.test.ts`. Then `bun nv viewer` to rebuild `viewer/dist/`, and the check
  `viewer` once.
- [x] `docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md`: the
  paragraph *A run is forgotten as a request is* says what § *Standing decisions* says, and its
  Declined list gets *a count of deleted rows per run*. `bun nv rules --render`.
- [x] `docs/rules/testing/a-run-is-recorded-like-a-request.md:28`: *The Runs view* says the page
  shows the cut.
- [x] `docs/examples/config/debug-index_max/about.md` and `docs/examples/config/debug-index_age/about.md`:
  a finished run's records are deleted one by one too. `crates/nvs-config/src/default.toml`'s
  `index_age` and `index_max` comments say the same.
- [x] `website/src/content/docs/tools/debug-viewer/index.mdx`: one sentence where the page describes
  a run. `website/src/data/changes.json:1`: the entry of § *Website*, written to AGENTS.md § *Text an
  end user reads*. Each page reread and stamped with `bun nv site --stamp`.

### 3. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written on 2026-11-26 by the loop session that built stage 1 (`8c0a7b2f00`). Stage 1 is done
and the check `rows` is green. `index.rs` deletes a run's rows by their own time and a run row
by its end on `runs_end`, `forget_rows` raises `meta`'s `cut_us`, and `SCHEMA` is 17. `/api/run`
returns `cut_us` (`runs.rs` `detail`). `viewer/src/api.ts`'s `RunDetail` already has
`cut_us: number | null`, and `viewer/test/fixtures/run.json` has `"cut_us": null`. The page
does not show it yet. `debug_api.rs` has `Service::start_on(data)`, which starts a service on a
data folder that `nvs debug --once` has already ingested.

### Next

Stage 2 from its first item: the note in `viewer/src/views/run.tsx:44` reads `detail.cut_us`.
Its browser case can set `cut_us` on a copy of `run.json` in `viewer/test/runs.browser.test.ts:20`.
No test is expected to go red.

### Backlog

- **The nightly's `serve` stack-page test on aarch64 is not this goal's, and nothing fixed it.**
  The four Miri jobs and the four conformance jobs of nightly run 45813627231 tested `4e43e1fa2e`.
  `c115a735a1` and `28a4d7a6bf` fixed both causes after that commit. The ninth job,
  `test / linux-aarch64 (tests)`, failed
  `serve::tests::a_waiting_request_holds_its_connection_and_its_isolate_on_the_heap`
  (`crates/nvs-cli/src/serve.rs:5555`) with 11.03 host pages per waiting request against
  `REQUEST_HOST_STACK_PAGES` 11. Measured on Windows in a debug build: (host, program) pages went
  (75, 0) → (340, 0) idle → (691, 102) waiting at `53e94a9181`, the last green nightly, and
  (75, 0) → (340, 0) → (691, 128) at `83eb71aaa2`. So the host part is 10.97 at both commits, not
  the 8.0 its doc comment records: it grew between `19466dc99c` (25 October) and `53e94a9181`.
  The program part grew from 3.2 to 4.0, exactly its bound of 4, in the commits after
  `53e94a9181`. The next step is a bisect over each range with that test and a printed reading.
  Do not share `CARGO_TARGET_DIR` with that worktree: see the playbook.
