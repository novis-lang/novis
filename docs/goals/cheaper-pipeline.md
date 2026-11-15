---
looper: 1
title: a CI run and a goal-end gate repeat no work they already did
summary: "Reached when the jobs that gate nothing run nightly, the jobs that only need the binary share one build, the pre-push checks run side by side and once per tree, and the recording step starts one process per atom."
context: [.github/workflows/ci.yml, tools/nv/cmd/ci.ts, tools/nv/cmd/verify.ts, docs/agent/commands.md, docs/rules/testing.md]
stages:
  - id: 1
    title: a job that gates nothing runs nightly
    status: done
    context: [.github/workflows/ci.yml, .github/workflows/build.yml, docs/rules/testing.md]
  - id: 2
    title: the jobs that only need the binary share one build
    status: done
    context: [.github/workflows/ci.yml]
  - id: 3
    title: the pre-push checks run side by side and once per tree
    status: done
    context: [tools/nv/cmd/ci.ts, tools/nv/cmd/verify.ts, tools/nv/select/change.ts]
  - id: 4
    title: a selection reads the tree once
    status: done
    context: [tools/nv/cmd/affected.ts, tools/nv/accept/runner.ts, tools/nv/select/select.ts, tools/nv/select/record.ts]
  - id: 5
    title: a recorded atom starts one process
    status: done
    context: [tools/nv/select/extract.ts, tools/nv/select/record.ts, tools/nv/select/store.ts]
  - id: 6
    title: the slow Rust tests share their set-up
    status: open
    context: [crates/nvs-ext/src/load.rs, crates/nvs-syntax/tests/prefixes.rs, crates/nvs-diagnostics/tests/every_code_is_tested.rs]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: pre-push-once
    stage: 3
    name: "the pre-push list runs its independent checks together, and a green tree is not checked twice"
    run: [bun, test, tools/nv/test/ci.test.ts]
    timeout: 600
    tests:
      - the checks that need no build start together
      - a push after a green check of the same tree runs no check again
  - id: one-profdata
    stage: 5
    name: "a recorded atom's functions are read with one profile process"
    run: [bun, test, tools/nv/test/extract.test.ts]
    timeout: 600
    tests:
      - one merge names the same functions the two calls named
---

# a CI run and a goal-end gate repeat no work they already did

## Why

Push run 43649226665 (green) was read job by job. It used 1,160 runner-minutes and 54.6 minutes of
wall time: Linux 733 minutes in 49 jobs, Windows 263 in 10, macOS 163 in 7. Set-up (checkout,
toolchain, cache) is 4 to 6% of each. Every cache hit. The time is compiling and testing.

- **Three jobs gate nothing and run on every push.** The unsafe audit took 33.6 minutes
  (`.github/workflows/ci.yml:1294`), the flood row 14.6 (`:1327`) and the optional musl build 11.6
  (`.github/workflows/build.yml:83`).
- **Six jobs rebuild `nvs` only to run it.** On Linux that is 38.7 minutes in the browser,
  extension-host, reference, valgrind, ext-templates and extension jobs
  (`.github/workflows/ci.yml:690`, `:939`, `:1219`, `:1528`, `:806`, `:877`). Windows adds 28.3 and
  macOS 18.3.
- **`wit-bindgen` is compiled from source in three jobs,** 9.6 minutes in all (`:803`).
- **The nightly repeats a push run.** Five of six recent nightlies ran a commit a push run had
  already tested.
- **The Linux x86_64 `tests` leg is the wall time, 53.1 minutes.** It runs the same 9,062 tests in
  2,249 s that aarch64 runs in 1,241 s. The cause is not known.

Locally, a goal end repeats itself:

- `prePush` awaits about 22 checks one after another (`tools/nv/cmd/ci.ts:825`), and `ci --push` runs
  the whole list again after `ci --check` (`:851`).
- `ci --check` repeats seven steps `bun nv verify` has just run (`tools/nv/cmd/ci.ts:289`).
- `bun nv affected --run` repeats its set-up up to five times: `cargo metadata`, about seven git
  calls and a 19.6 MB parse each (`tools/nv/accept/runner.ts:221`, `tools/nv/cmd/affected.ts:187`).
- Recording starts two `llvm-profdata` processes per atom, about 22,000 on a full run
  (`tools/nv/select/extract.ts:125`), and a case batch waits for its extraction before the next
  batch starts (`tools/nv/select/record.ts:444`).
- About 200 `nvs-ext` tests each compile a built-in component from scratch
  (`crates/nvs-ext/src/load.rs:354`), because every test is its own process.

## Standing decisions

**Limits: what stops this goal.**

- **CI is measured by reading one run, never by starting one.** A stage that edits a workflow names
  the minutes it expects to save. The goal end's push is the one run that shows them. A session
  never pushes to measure.
- **A workflow change that the goal end's run shows red is reverted, not repaired twice.** One fix
  attempt is allowed. If the same job is red again, the change is taken out and recorded in
  `### Backlog`.
- **An item gets one attempt, and a stage two sessions,** counted in `### State`. At the ceiling the
  open items move to `### Backlog` as declined.
- **A local change is kept when a count drops:** processes started, checks run, git or cargo calls.
  It is not judged by seconds. No full verify, sweep or proof run is started to measure anything.
- **Selection never narrows.** No change in this goal may make a check run less often than its
  footprint asks. An item that would is declined.
- **No item is added.**

**The three jobs that gate nothing move to `schedule` and `workflow_dispatch`.**
`rule:testing/every-run-runs-every-job` gains the sentence that a job whose result gates nothing
runs nightly. **What it spends:** an unsafe count, a flood row or a musl break is seen up to a day
later. None of them can turn a run red today.

**One debug `nvs` per OS is built once and handed to the six jobs as an artifact.** The `tests` and
conformance legs keep their own build, because they are the critical path. Safe default if the
upload and downloads on Linux cost more than the 38.7 minutes they replace: keep the artifact on
Windows and macOS only.

**The nightly runs only what reads the outside world when the commit already has a green push
run.** That is the supply-chain job, the nightly-toolchain jobs and the image pulls. **What it
spends:** on a quiet day a changed runner image is found by the next push, not by the nightly.

**Miri and the sanitizers still run on every push.** Skipping them by changed crate would make a
skipped job count as green, which `release --tested` cannot accept. Declined.

**The Windows `tests` leg keeps its three partitions.**

**The x86_64 gap gets one probe session.** Stage 6 reads the per-test times of the measured run with
`gh`. If the `nvs-ext` component compiles explain it, the cache item fixes it. If not, the finding
is one line in `### Backlog` and nothing more is tried.

**The `nvs-ext` test cache is a file cache under `target/test-scratch/`,** through the existing
`ModuleCache` trait, keyed as the product's cache is. The first process compiles and the others
deserialize. **What it spends:** the artifacts' size on disk, which `bun nv disk` counts.

**Landings to `tools/nv/lib/reads.ts` or `tools/nv/proofs/` are batched into one commit per stage,**
because each reruns every check once.

**Lanes:** stages 1 and 2 share `ci.yml` and run in order. Stages 3 to 5 share `tools/nv/` and run
in order. Stage 6 touches only `crates/` and can run as a side goal.

## Stages

### 1. a job that gates nothing runs nightly

**Does:** move three jobs off the push path, install `wit-bindgen` prebuilt, and take the bench and
doc tests off the slowest leg.

- [x] `.github/workflows/ci.yml:1294`: `unsafe-audit` runs on `schedule` and `workflow_dispatch`,
  and leaves the `needs` list at `:1740` (`rule:testing/every-run-runs-every-job`).
- [x] `.github/workflows/ci.yml:1327`: the flood job, the same way
  (`rule:testing/every-run-runs-every-job`).
- [x] `.github/workflows/build.yml:83`: the musl leg builds in the nightly and in `release.yml` only
  (`rule:testing/every-run-runs-every-job`).
- [x] `docs/rules/testing.md:1`: the rule's fragment gains the sentence, and `bun nv rules --render`
  writes the chapter (`rule:testing/every-run-runs-every-job`).
- [x] `.github/workflows/ci.yml:803`: `wit-bindgen` 0.63.0 is downloaded and checked by SHA-256, as
  the nextest step is (`rule:testing/every-run-runs-every-job`).
- [x] `.github/workflows/ci.yml:303`: the bench and doc tests run in the Linux `conformance 2/2` leg
  (`rule:testing/every-run-runs-every-job`).

### 2. the jobs that only need the binary share one build

**Does:** build a debug `nvs` once per OS and download it in the six jobs, and stop the nightly from
repeating a tested commit.

**Split:** an `Explore` subagent lists every `cargo` call the six jobs' `bun nv` commands make
themselves, as `file.ts:NN`, before the first edit.

- [x] `.github/workflows/ci.yml:835`: a `nvs-debug` job per OS uploads the binary, and the jobs at
  `:703`, `:907`, `:969`, `:1248` and `:1563` download it
  (`rule:testing/every-run-runs-every-job`).
- [x] `.github/workflows/ci.yml:13`: a scheduled run whose commit has a green push run from the last
  day runs the outside-world jobs only (`rule:testing/every-run-runs-every-job`).

### 3. the pre-push checks run side by side and once per tree

**Does:** run the checks that need no build together, remember a green tree, and skip what verify
has just proved.

- [x] `tools/nv/cmd/ci.ts:825`: the checks ahead of `cargo doc` start together, bounded, and print
  in list order (`rule:testing/perf-two-mechanisms`).
- [x] `tools/nv/cmd/ci.ts:851`: a green `--check` writes a stamp of `HEAD` and the tree signature of
  `tools/nv/select/change.ts:132` under `.agent-tmp/ci/`, and `--push` skips a check stamped green
  for that exact tree (`rule:testing/perf-two-mechanisms`).
- [x] `tools/nv/cmd/ci.ts:289`: a check whose step atom is green and unselected in the store is
  skipped, as `tools/nv/cmd/verify.ts:349` decides it. `chapter examples` is not skipped, because it
  runs another binary (`rule:testing/perf-two-mechanisms`).
- [x] `tools/nv/test/ci.test.ts:1`: the two tests of the check `pre-push-once`.

### 4. a selection reads the tree once

**Does:** pass the graph and the change from one planning step to the next, and cut the repeated git
calls and stats.

- [x] `tools/nv/accept/runner.ts:221`: `PlanSweep.open` takes the graph and the change that
  `tools/nv/cmd/affected.ts:187` read (`rule:testing/perf-two-mechanisms`).
- [x] `tools/nv/select/select.ts:356`: one `rev-parse` for the three revisions, and one
  `ls-files --others` shared with `tools/nv/select/change.ts:124`
  (`rule:testing/perf-two-mechanisms`).
- [x] `tools/nv/select/select.ts:771`: `discover` marks cases, proofs, benches and Novis tests
  complete, so presence is a set lookup (`rule:testing/perf-two-mechanisms`).
- [x] `tools/nv/cmd/verify.ts:600`: the cache parent is made private once and each binary's folder
  inherits it, through `tools/nv/select/record.ts:50` (`rule:testing/perf-two-mechanisms`).

### 5. a recorded atom starts one process

**Does:** read an atom's functions from one `llvm-profdata` call, overlap a batch's extraction with
the next batch's run, and write a batch in one transaction.

- [x] `tools/nv/select/extract.ts:125`: `merge -sparse -text` names the functions, after the test
  `one merge names the same functions the two calls named` passes on ten recorded atoms
  (`rule:testing/perf-two-mechanisms`). Safe default if the names differ on any atom: decline.
- [x] `tools/nv/select/record.ts:452`: batch n+1 starts while batch n is extracted, each batch in
  its own folder (`rule:testing/perf-two-mechanisms`).
- [x] `tools/nv/select/record.ts:509`: a batch's verdicts and skip slots are one transaction of
  `tools/nv/select/store.ts:480` (`rule:testing/perf-two-mechanisms`).

### 6. the slow Rust tests share their set-up

**Does:** give the `nvs-ext` tests a file cache, thread two serial tests, and spend one session on
the x86_64 gap.

- [ ] `crates/nvs-ext/src/load.rs:266`: a file-backed `ModuleCache` for tests under
  `target/test-scratch/`, used by `crates/nvs-ext/tests/spreadsheet_read.rs:86` and the six other
  files that call `Loader::new` (`rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`).
- [ ] `crates/nvs-syntax/tests/prefixes.rs:112`: the three sweeps use the worker pool of
  `crates/nvs-syntax/tests/lossless.rs:125` (`rule:testing/perf-two-mechanisms`).
- [ ] `crates/nvs-diagnostics/tests/every_code_is_tested.rs:46`: one scan of the text into a set,
  then one lookup per code (`rule:testing/perf-two-mechanisms`).
- [ ] `.github/workflows/ci.yml:124`: the probe of the x86_64 leg, read with `gh` from run
  43649226665, written as one paragraph in `## Standing decisions`
  (`rule:testing/every-run-runs-every-job`).

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.
- [ ] `docs/agent/commands.md:1`: after the goal end's push, read that run with
  `bun nv ci --status` and write its runner-minutes by OS beside the 1,160 above. A job this goal
  changed that is red gets one fix, then a revert.

## Handoff

### State

Stages 1 to 5 are done. Sessions used: stage 1: 1, stage 2: 1, stage 3: 1, stage 4: 1, stage 5:
1, stage 6: 0.

- Stage 5: one `merge -sparse -text` names an atom's functions (`namesFromText`); the `one-profdata`
  test matched merge then `show` over ten atoms. `llvm-cov` reads only an indexed profile, so
  `CovMap.ensure` takes a function that merges one on a cache miss. `recordCases` extracts batch n-1
  while batch n runs, in `cases-<n>`, and records a batch in one `store.transaction`.
- Stage 4: `commitsOf` and `untrackedPaths` in `change.ts`; `discover` lists atoms in full.
- Stage 3: `--push` skips checks when `STAMP` names the pushed tree. `pool` is in `lib/proc.ts`.
- `nvs-debug` (`.github/workflows/ci.yml:703`) feeds six jobs per OS. If a browser suite goes red on
  the push for a feature only the workspace build unified, that is the cause.
- `tested` (`:97`) runs in the nightly alone; `tools/nv/test/ci-push-builds.test.ts` pins the kept
  list. `build.yml`'s matrix expressions first run on GitHub at the push.
- Minutes the goal end's push should show saved: stage 1's 34, 15, 12 and 9, the bench and doc
  tests' share of the 53.1-minute wall, and stage 2's 38.7 less the transfers. If the Linux transfer
  costs more, keep the artifact on Windows and macOS only.
- The sweep's `env-secret-proofs` is stale looper state: nothing names it, and no log exists.

### Next

Stage 6, in one session. It touches only `crates/`.

- [ ] `crates/nvs-ext/src/load.rs:266` (`ModuleCache`): a file-backed cache for tests under
  `target/test-scratch/`, used by `crates/nvs-ext/tests/spreadsheet_read.rs` and the six other files
  that call `Loader::new`.
- [ ] `crates/nvs-syntax/tests/prefixes.rs:118`, `:152` and `:225`: the three sweeps use the worker
  pool of `crates/nvs-syntax/tests/lossless.rs`.
- [ ] `crates/nvs-diagnostics/tests/every_code_is_tested.rs:80` (`asserted`): one scan of the text
  into a set, then one lookup per code.
- [ ] `.github/workflows/ci.yml:142`: read run 43649226665's per-test times of the x86_64 leg with
  `gh`, one paragraph in `## Standing decisions`.

No test is expected to go red. `select-record.test.ts`'s `CovMap` test now passes `ensure` a
function. `tsc` over the tools is clean.

### Backlog

- Declined before the run: larger runners, a workspace-hack crate, merging integration tests into
  one binary per crate, and two Windows `tests` partitions.
- Declined before the run: running the `--probes` and checked-types conformance passes on Linux
  only. Each target is its own backend.
