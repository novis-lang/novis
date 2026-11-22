---
looper: 1
title: a push run of CI ends sooner and runs every job it runs today
summary: "Reached when bun nv ci --times prints every job's wait and run minutes, the Windows test legs carry the benches and the cache save on their lightest slice, the Windows proofs leg runs the roster in two parts with bun nv proofs --part, the dev build starts after the debug binaries so every macOS leg gets a runner at the start, the slowest proof programs wait only where the wait is what they prove, and a push run of the new layout is green with its times recorded."
context: [.github/workflows/ci.yml, docs/rules/testing/every-run-runs-every-job.md]
stages:
  - id: 1
    title: "bun nv ci --times prints each job's wait, run time and slow steps"
    status: done
    context: [tools/nv/cmd/ci.ts, tools/nv/test/ci.test.ts]
  - id: 2
    title: "bun nv proofs --part k/n runs one part of the roster"
    status: done
    context: [tools/nv/cmd/proofs.ts, tools/nv/proofs/run.ts, tools/nv/test/proof-command.test.ts]
  - id: 3
    title: "the new layout: lighter Windows test leg, Windows proofs in two parts, the dev build after the debug binaries"
    status: done
    context: [.github/workflows/build.yml]
  - id: 4
    title: "the layout is pushed, its run is green, and its times are recorded"
    status: done
    context: [docs/goals/ci-green.md]
  - id: 5
    title: the slowest proof programs wait only where the wait is what they prove
    status: done
    context: [tools/nv/proofs/run.ts, docs/rules/testing/feature-proofs.md, docs/agent/prose.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: ci-times
    stage: 1
    name: "bun nv ci --times prints each job's wait, its run time, its steps over a minute, and the job that ended last"
    run: [bun, test, tools/nv/test/ci.test.ts, -t, "bun nv ci --times"]
    timeout: 300
  - id: proofs-part
    stage: 2
    name: "bun nv proofs --part gives every program of the run to exactly one part, and refuses a part outside 1..n"
    run: [bun, test, tools/nv/test/proof-command.test.ts, -t, "--part"]
    timeout: 300
  - id: layout-green
    stage: 4
    name: "origin/main holds the new layout, and its push runs of CI and Website ended green"
    run: [bun, nv, ci, --green]
    timeout: 300
  - id: close
    stage: 6
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, ci-critical-path]
    timeout: 900
---

# a push run of CI ends sooner and runs every job it runs today

## Why

A push run of `ci.yml` takes about 46 minutes. Run 44940832132 (commit `a477b5804`) is the baseline
every figure below comes from. The jobs run side by side, so the run ends when its last job ends.
Four jobs end within a few minutes of each other:

| Job | Waits | Runs | Ends at | Where the time goes |
|---|---|---|---|---|
| `proofs / windows-x86_64` | 0 | 45 | 45 | proof build 10, forced unwind 1, feature proofs 32 |
| `test / macos-aarch64 (tests)` | 6.7 | 37 | 43.8 | build 16, nextest 16, benches 2.4 |
| `test / windows-x86_64 (tests 1/3)` | 0 | 40 | 40 | build 21, nextest 11, benches 3.5, cache save 2.9 |
| `proofs / macos-aarch64` | 0 | 39 | 39 | proof build 9, forced unwind 1, feature proofs 28 |

Below them sit `miri units (nvs-stdlib 11/12)` at 35, Windows conformance at 33 and Windows cost
guards at 32. That is the floor this goal does not move.

Each of the three slow causes has a fix that costs nothing but the work. The user's call: a saving
with no trade-off beyond the one-time work is taken.

- **The Windows `tests` legs are unequal.** Slice 1 runs nextest for 664 s, and slices 2 and 3 for
  460 s and 430 s. Slice 1 also runs the benches and doc tests (`ci.yml:348`, `:352`) and is the one
  leg that saves the cache (`ci.yml:243`). Moving those three to the lightest slice takes about 6
  minutes off the longest leg.
- **Windows feature proofs is one step of 32 minutes.** It runs two programs at a time, so that every
  run judges a program at the same load (`ci.yml:582`). Two parts of the roster, each at two programs,
  keep that load and halve the step. Linux and Windows runners have no limit this run reaches: every
  Linux and Windows job that needs only `tested` started within 6 seconds of the run.
- **macOS has five runners, and the `test (tests)` leg waits for one.** At the start, the five go to
  cost guards, proofs, conformance, `debug nvs` and the dev build's macOS leg. The tests leg gets
  the runner `debug nvs` frees, 6.7 minutes in. The dev build (`ci.yml:1844`) gates nothing in the
  run itself: the `dev` image waits for the whole run to be green. If it starts after `debug nvs`, it
  still ends at about 30 minutes, and the tests leg starts at once.

With all three, the last job is `proofs / macos-aarch64` at about 39 to 40 minutes. A second macOS
proofs part does not help: it would wait for a sixth runner and end later than the one part does. So
macOS proofs are made shorter by stage 5 only, which makes the slowest programs faster on every OS.

No figure here is a target. The run's times are recorded, and no check reads a minute.

This goal moves no feature, so it owes the website nothing. Stage 5 may change proof programs: each
one it changes is reread and stamped like any edit to a published `.nvs` file.

## Standing decisions

**Every job that runs today still runs, with the same steps, on the same OS.** No test, proof,
step or leg is removed, skipped, made optional or moved to the nightly. Only where a step runs and
when a job starts may change.

**`bun nv ci --times [<run id>]` is the one way a session reads a run's times.** With no id, it reads
the newest complete push run of `ci.yml` on `main`. It prints one line per job, sorted by the minute it
ended: its wait (its start minus the run's creation), its run time and its end, in minutes with one
decimal. Under each job, every step that took a minute or more. The last line names the job that
ended last. It reads `gh run view <id> --json createdAt,jobs`, and its test reads a stored JSON
fixture, never the network. Reason: rule 12, "a manual step done twice becomes a command". This goal
reads times four times, and the next goal reads them too.

**`bun nv proofs --part K/N` takes every N-th program, round-robin, starting at the K-th.** The
programs are the run's list after the gate (`tools/nv/cmd/proofs.ts:425`), sorted by kind and then
by path, so the split is the same on every OS. The gate runs whole in every part, because it is cheap
and a part that skipped it could pass alone. `N` is 1 to 8, and `K` is 1 to `N`. A program two scopes
list is still one program and lands in one part. It works with `--run` and `--verify` and with nothing
else. Declined: a split by recorded time. It needs a ledger that every new program misses. Safe default if the two parts differ by more
than 3 minutes on the run of stage 4: leave it, and write the figures in the handoff.

**Only Windows runs proofs in two parts.** Linux ends at 31 minutes in one part, with no gain from a
second. macOS has no free runner for a second part. The matrix gains a `part` value. Linux and macOS
rows have none and run as today. The steps that run once per OS run in Windows part 1: the forced
unwind test and the website stale guard. Part 1 saves the cache, and part 2 reads it. Both parts
build the proof binary themselves. Declined: one build job that hands the binary to both parts. It
saves runner minutes, but the parts would wait for it, so the run would end later.

**The Windows `tests` slice that runs the benches, the doc tests and the cache save is the lightest
one.** The session reads the nextest step of the last three green push runs with `bun nv ci --times
<id>` and picks the slice with the smallest total. Over runs 44945274371, 44940832132 and
44892963091 it is slice 2: 18.9 minutes, against 26.3 for slice 1 and 22.3 for slice 3. The `if:` at
`ci.yml:348` and `:352` and the `save-if` at `:243` name that slice for Windows. Every other row keeps
its current condition: Linux x86_64 runs them in `conformance 2/2`, and the single-leg rows in their
`tests` leg.

**The dev build starts after the debug binaries.** The `build` job (`ci.yml:1844`) gets `needs:
[tested, nvs-debug]` and keeps its `if: ${{ !cancelled() }}`. In the release, `release.yml` calls
`build.yml` itself, so the release is unchanged. Before the change, the session checks with `bun nv
ci --times` that each build leg, started at the minute `debug nvs` ends, still ends before the run's
last job. Safe default if one does not: the build leg that ends latest decides, and the change is
dropped if any leg would end after the last job.

**Each Windows and Linux proofs leg writes its program times.** It sets `NVS_PROOF_TIMES`
(`tools/nv/proofs/run.ts:1341`) and uploads the file as the artifact `proof-times-<name>-<part>`, kept
7 days, with `if: always()`. It costs one small file per leg. Stage 5 reads it, and so can anyone
looking for slow programs later.

**A slow proof program is made shorter only where the wait is not what it proves.** A `sleep` that
waits for a server, a poll on a timer or a timeout longer than the test needs is replaced by an
event the program can wait on, as testing.md § *A test holds on every CI runner* says. A program that
proves a timeout, a deadline or a rate keeps its wait. A program's expected output is never changed to
make it pass. Each changed program keeps its comments to AGENTS.md § *Text an end user reads*.

**The comment above each job in `ci.yml` is the home of its layout.** Each change rewrites that
comment as a whole, with what it spends and why. `rule:testing/every-run-runs-every-job` is unchanged:
it names no job's layout.

**Pushing.** Stage 4 pushes with `bun nv ci --check`, then `bun nv ci --push`, and watches with `bun
nv ci --watch --next`, as `ci-green`'s decisions say. Never `git push`, never a force push, never
another branch, and no release.

**What stops it.** One attempt per item, and two sessions per stage, counted in `### State`. A red job
that the layout causes and one fix does not clear is reverted for that job alone, and the figures
go in the handoff. In stage 5, one attempt per program: one that does not get shorter is noted
and left as it is. No minute holds the goal open.

**Lanes.** Stages 1 and 2 touch disjoint files and may run side by side. Stage 3 needs both: it
names `--part`, and its check reads `--times`. Stages 3 to 5 run in order.

## Stages

### 1. bun nv ci --times prints each job's wait, run time and slow steps

**Does:** adds `--times [<run id>]` to `bun nv ci`.

- [x] `tools/nv/cmd/ci.ts:169`: `--times` in the usage lines and the file's head comment, as the
  standing decision gives it.
- [x] `tools/nv/cmd/ci.ts:1`: the reader and the printer. The newest complete push run of `ci.yml` on
  `main` when no id is given. A run still going prints `run <id> is still going` and exits 1.
- [x] `tools/nv/test/ci.test.ts:1`: a `describe("bun nv ci --times")` over a stored fixture under
  `tools/nv/test/fixtures/`. It is cut down from run 44940832132's JSON to a handful of jobs, with
  neutral names kept as GitHub gave them. It checks the wait, the run, the end, the steps over a minute
  and the last line. It runs once, by name.

### 2. bun nv proofs --part k/n runs one part of the roster

**Does:** adds `--part K/N` to `bun nv proofs`.

- [x] `tools/nv/cmd/proofs.ts:87` and `:90`: `--part K/N` in the help and the usage. `tools/nv/cmd/proofs.ts:1`:
  one line in the head comment.
- [x] `tools/nv/cmd/proofs.ts:466`: the argument is parsed, and refused outside `1 <= K <= N <= 8`, or
  without `--run` or `--verify`.
- [x] `tools/nv/cmd/proofs.ts:425`: `programs` is sorted by kind and path and cut to the part before
  it runs. The gate before it is not cut. The summary line names the part: `part 1/2: 653 programs`.
- [x] `tools/nv/test/proof-command.test.ts:1`: a test named for `--part` that the union of the parts
  is the list, no program is in two parts, and `0/2`, `3/2` and `1/9` are refused. It runs once, by name.

### 3. the new layout: lighter Windows test leg, Windows proofs in two parts, the dev build after the debug binaries

**Does:** changes `ci.yml` as the standing decisions say. Gate: `bun nv ci --times --help` and `bun nv
proofs --help` both name their new flags.

- [x] Read the last three green push runs with `bun nv ci --times <id>`, and write each Windows slice's
  nextest total in the handoff. Pick the lightest.
- [x] `ci.yml:243`, `:348` and `:352`: the benches, the doc tests and the cache save go to that
  Windows slice. Every other row is unchanged. The comment at `ci.yml:339` says which slice and why.
- [x] `ci.yml:527`: the proofs matrix has two Windows rows with `part: 1` and `part: 2`. The job name
  shows the part. `ci.yml:543`: the cache key stays per OS, and only part 1 saves.
- [x] `ci.yml:579` and `:610`: the forced unwind test and the website stale guard run on every row
  but Windows part 2. `ci.yml:584`: the step passes `--part <k>/2` on Windows and nothing elsewhere.
  It sets `NVS_PROOF_TIMES`, and a step after it uploads the file as the decision says.
- [x] `ci.yml:510`: the job's comment rewritten as a whole: the parts, what they spend (one more
  Windows runner and one more proof build) and why macOS has one part.
- [x] `ci.yml:1844`: `build` gets `needs: [tested, nvs-debug]`, after the check the decision names.
  Its comment says why it waits.
- [x] `bun nv ci --check` is green. It lints the workflow files with the rest.

### 4. the layout is pushed, its run is green, and its times are recorded

**Does:** pushes once, watches the run and fixes what the layout breaks.

- [x] `bun nv ci --check`, then `bun nv ci --push`. Then `bun nv ci --watch --next` with
  `run_in_background` and a timeout of 7200000, and end the turn.
- [x] A red job is fixed as `ci-green`'s decisions say, one push for every fix of a run. A red job the
  layout causes gets one fix, and if it is still red it is reverted for that job.
- [x] `bun nv ci --green` exits 0.
- [x] `bun nv ci --times` on that run: the table in the handoff's `### State`, with the baseline's
  ending minute beside each job, and the two Windows proofs parts' run times.

### 5. the slowest proof programs wait only where the wait is what they prove

**Does:** reads the program times from stage 4's run and shortens the programs that wait on a timer.

**Split:** the programs that need a change go to writing subagents on the session's model, in
batches of disjoint files. Each one gets its program, its time on each OS and the decision above. The
session builds once they end and runs the changed programs by name.

- [x] `gh run download <id> -p 'proof-times-*'` into `.agent-tmp/proof-times/`. List the 30 programs
  with the longest Windows time, with their Linux time beside each, in the handoff.
- [x] For each of the 30, one line in the handoff: the wait it has, and whether that wait is what it
  proves. A `reader` subagent reads the 30 programs and reports `path:line` and one line each.
- [x] Each program whose wait is not what it proves is changed. Each changed program runs once with
  `bun nv proofs --only <feature> --run`.
- [x] Each changed published program is reread for its comments, and `bun nv site --check stale` is
  green. `.agent-tmp/proof-times/` is deleted.

### 6. the goal is verified once, and every failure is fixed

- [x] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [x] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Every stage is done, one session each and stage 4 in two. The goal is reached.

- Stage 6, a close: `cargo build`, `cargo clippy --all-targets -- -D warnings`, `ci-times`,
  `proofs-part` and `bun nv close --goal ci-critical-path` are green, none red on the first run.
- Its decisions have homes: the layout in `ci.yml`'s job comments, `--times` in `tools/nv/cmd/ci.ts`'s
  header, `--part` and its Declined in `tools/nv/cmd/proofs.ts`'s header. Its Backlog moved to
  `ci-green`'s Backlog.

- Push 2, `0adafda88`, run 44980678443, is green (`bun nv ci --green`). It ended at 53.9 min, last
  `proofs / linux-x86_64`, 53.8 (34.2 in push 1, 31.6 in the baseline 44940832132). Windows proofs
  parts 41.5 and 40.6. Push 1 ended at 39.3 against the baseline's 45.9. Build legs end before the last job.
- The Linux leg's 53.8 against 34.2 is a slower runner, not a wait: its programs total 72.3 min
  against 48.8, and every program is about 1.5 times slower.
- Of the 30 slowest Windows programs, only `hostile/core/Net-Stream/read/01` waited where the wait
  is not what it proves: now `1us` and `100ms` reads, 17.5 s became 0.9 s. The `Http-Client`
  attacks wait on Windows name resolution; the rest is file, database or image work.
- Nightly 44894790665 ran `7e97fa8444`, which does not hold the Miri fix `156f11a7ca`
  (`git merge-base --is-ancestor`). Push 2's Miri jobs are green.
- `website/src/data/roadmap.json` lists `ci-critical-path` and `windows-rust-lld` as `internal`.
- The first DONE was refused: `layout-green` was red because stages 4 to 6 were never pushed.
  `bun nv ci --check` and `bun nv ci --push` pushed `90ddac8b1`. Its CI run 44987257192 and Website
  run 44987257202 are green, and `bun nv ci --green` exits 0. This handoff commit touches only
  `docs/goals/`, so `--green` still reads `90ddac8b1` as the built commit.

### Next

The goal is reached: the driver runs its acceptance and retires it, then `windows-rust-lld` is live.
No test is expected to go red.

### Backlog

- Both items, the `Core\Http\Client` deadline that does not bound name resolution and the image
  programs slow under the proofs pool, are in `docs/goals/ci-green.md`'s Backlog.
