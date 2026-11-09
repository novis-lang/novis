---
looper: 1
title: main's tip is ready to release
summary: "Never reached: each time a person starts its loop, it runs every CI job that can run on this machine, fixes every failure in one pass, pushes, and fixes and pushes again until that push's CI run is green. A green push run is the full suite, the dev binaries and the dev image, so the commit is ready to release. Then the loop stops itself."
context: [tools/nv/cmd/ci.ts, tools/nv/cmd/fuzz.ts, tools/nv/cmd/release.ts, .github/workflows/ci.yml, .github/workflows/dev-build.yml, .github/workflows/release.yml, looper.toml]
stages:
  - id: 1
    title: one command runs every CI job this machine can run
    status: open
    context: [tools/nv/cmd/fuzz.ts, tools/nv/cmd/linux-test.ts, .github/workflows/ci.yml]
  - id: 2
    title: one push run is the full suite, the dev build and the release's proof
    status: open
    context: [.github/workflows/ci.yml, .github/workflows/dev-build.yml, .github/workflows/build.yml, .github/workflows/image.yml, .github/workflows/release.yml, tools/nv/cmd/ci.ts, tools/nv/cmd/ci-changes.ts, tools/nv/cmd/release.ts]
  - id: 3
    title: the release cycle
    status: open
    context: [tools/nv/cmd/ci.ts, .github/workflows/ci.yml]
checks: []
---

# main's tip is ready to release

## Why

Three `Release` runs in a row went red on one day, each on a bug that no push had tested: three
round trips that `fuzz smoke / markdown, markdown_from_html` found, Miri tests too large for Miri's
limit, and a CPU-time stop that wrote no sweep line on Windows. Each red cost a 75-minute run, and
each run found only the next bug. A push ran only the jobs its diff reached, and Miri, the fuzzers and
the full proofs ran nightly and at release alone. So the first place they ran on new code was the
release.

Two changes move that first run earlier.

- **Every CI job that can run on this machine runs here first** (`bun nv deep`, stage 1). Every
  failure it finds is fixed in one pass before anything is pushed.
- **Every push runs everything** (stage 2). The full suite took the same wall time as the push lane,
  about 50 minutes, because its longest job runs in both. It costs about twice the runner minutes,
  which a public repository does not pay for. The same run builds the dev binaries and publishes the
  `dev` image, as the separate `Dev build` workflow does now. So a green push run tests everything a
  release dry run tests, and a real `Release` of that commit skips its `test` job.

A real `Release` still builds its own binaries, about 20 minutes. A release binary has its version
compiled in (`nvs --version` prints `CARGO_PKG_VERSION`, and the binary carries that version's
`CHANGELOG.md` section). That version exists only once a person picks the bump, and a push binary
never claims a release's version (`build.yml`'s version check). So a release of a green push takes
about 30 minutes: plan, build, the approval, the tag, the draft and the images.

This goal is never reached. A person starts its loop with `bun tools/looper/main.ts loop start
--loop side-release-green`, and the loop stops itself once the push run is green.

## Standing decisions

**The loop works in the root, on `main`.** It is added with `bun tools/looper/main.ts loop add --side
release-green --in-root`, so it has no worktree, no branch and no landing. The main loop's sessions
edit the same tree at the same time, so every rule for two sessions in one tree holds: commit with a
pathspec of your own files, never run a tree-wide git command (`stash`, `checkout -- .`, `reset`,
`add -A`, `clean`), and leave every file you did not write as it is. The claims guard refuses a file
another session holds.

**`bun nv deep` is the local lane: every `ci.yml` job this machine can run, in one call.** It is new
in `tools/nv/cmd/deep.ts`, whose `steps()` is the map and whose `CI_ONLY` lists each job or leg with
no local step and its reason; `bun nv deep --list` prints both. Each step runs the job's own `run:`
lines, on Windows or in WSL's copy of the tree where the job runs on Linux, and a step's `note` says
where it departs. `tools/nv/test/deep.test.ts` fails when `ci.yml` has a job neither names, so a new CI
job is never skipped unseen. What stage 1 settled while building it:
- **Every step copies its job's commands, not a `bun nv` stand-in.** `test` runs nextest, the doc and
  bench tests and the four conformance runs, on Windows and in WSL, and `lint`, `attribution` and
  `docs` run their own lines. `bun nv verify --full` only stops truncating output, and `bun nv ci
  --check` skips clippy, rustdoc and half the docs job, so neither covers its job.
- **The Linux suite runs through `deep.ts`'s own WSL runner** (`wslLine`), not `bun nv linux-test`,
  which keeps refusing an unfiltered run for a person at the keyboard.
- **A debug build here never takes `-p`.** A step builds the whole tree and runs the built binary where
  the job runs `cargo run -p nvs-cli`. Release, proof-profile, Miri and sanitizer lines keep the job's
  `-p`, each in its own target directory.
- **`lint`'s clippy and rustdoc run in WSL**, so code under `cfg(unix)` is linted, as on CI's runner.
- **`docker` builds its Linux binary in a `rust:1.99.1-bookworm` container**, since WSL's glibc 2.39 is
  newer than the images'. Its cargo registry, toolchain and target directory are Docker volumes.
- **`unsafe-audit` is CI-only**: it writes cargo-geiger's counts to the run summary and is never red.
- **Miri's units run each crate whole**, where CI splits them into 14 shards.
- **A step checks the programs it needs first** (`TOOLS`), and a missing one makes it red with its
  install line, which `docs/agent/commands.md` § *What `bun nv deep` needs installed* repeats.
- Stage 2's rows (the build legs, the `dev` image and the `--apply` check) are added to `steps()` with
  the jobs that stage adds to `ci.yml`; the coverage test fails until they are.

**`bun nv deep` runs every step and stops at none.** It prints one line per step as it ends, and at the
end each red step with its command and the tail of its log. It writes the whole report to
`.agent-tmp/deep/` and exits 1 when a step is red. `--failed` runs again only the steps the last report
names red. `--only <job>` runs one job's step. One step runs at a time, so a timing guard never shares
the machine with a fuzzer or Miri. What it costs: about four to five hours of this machine per full
run, measured in stage 1. The first run's steps took two hours without `fuzz-smoke` (about 90 minutes
while two targets are young), without the three steps whose WSL tools are missing, and with `asan`,
`tsan` and `miri-units` stopping at their first red crate. The longest are `miri` (31 min),
`test-windows` (22 min) and `cost-guards` (21 min, both legs). A session's background call ends at two
hours, so a full run is taken in parts with `--only` and `--failed`. That is still less than one red
CI run plus its fix and its second run.

**`bun nv fuzz --all` keeps fuzzing after a crash.** It runs libFuzzer's fork mode with
`-ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1`, and at the end lists every input it saved,
each with its base64. So one run finds every crash it can, and all of them are fixed in one pass.
Each target runs `120` seconds on the fork mode's jobs, about three times the work of the CI job's
300 seconds on one core. A target whose `fuzz/fuzz_targets/<target>.rs` is younger than 14 days runs
`1800` seconds, because a young target finds a new bug every few minutes. `markdown_comrak` stays out,
as it does in CI.

**Every run of `ci.yml` runs every job.** Stage 2 deletes the lanes: the `changes` job, `bun nv
ci-changes`, the `full` input and the `deep` output, and the rules `testing/ci-lanes`,
`testing/lane-table` and `testing/the-deep-lane`, each rewritten or deleted with every citation of it.
A push, a pull request, the nightly, a dispatch and a release's `test` job run the same jobs.
- **What it costs:** about 1,030 to 1,280 runner minutes per run, against about 620 for a push lane
  run (measured on two push runs and two nightlies). The extra jobs are Linux ones, so the macOS
  jobs stay as many. On a busy day, runs may wait longer behind GitHub's limit on jobs at once. A
  push already cancels the older push's run on `main` (`rule:testing/one-run-on-main`).
- **The nightly stays,** on the same jobs. It catches what changes outside the tree on a day with no
  push: a new advisory, a new nightly toolchain, a download that moved.

**The `Dev build` workflow merges into `ci.yml`.** Its build legs become `ci.yml` jobs that call
`build.yml` with no version, beside the tests, and its `dev` image becomes a job that calls
`image.yml` and needs every other job green. `dev-build.yml` is deleted, with its `workflow_run`
trigger, its two concurrency groups and its `check` job. Every push builds: the skip for a push that
changed nothing the build reads goes with `ci-changes`. The `build.yml` legs ran on every such push
already, so the merge adds no build. `bun nv ci`'s workflow lists (`CALLS_BUILD`,
`REPLACED_BY_RELEASE`, the watched workflows) and their tests lose `Dev build`.

**Each push also proves the release's version step, without a second build.** A Linux job runs `bun
nv release --apply` with the next patch version in its own working copy, as `build.yml` does before a
release build, and then `cargo metadata --locked` and the fuzz workspace's lock update. It catches
what a versioned build breaks before it compiles: the manifest rewrite, the separate lockfiles and the
changelog section. It writes nothing back.

**A green full push run is the release's proof.** `bun nv release --tested` accepts a green `ci.yml`
run of `main` at the commit, from any event, since every run is full now (`FULL_CI_EVENTS` in
`tools/nv/cmd/release.ts` goes). `bun nv ci --dispatch release` refuses unless `main`'s tip has such a
run, in place of a green dry run. The dry run stays in `release.yml` as a person's tool, for a change
to the release workflows themselves, and this goal never starts one. `docs/release.md` says the new
order.

**The session pushes only after a green local run.** `bun nv ci --check`, then `bun nv ci --push`,
then `bun nv ci --watch`. Never `git push`, never a force push, and never a push of another branch. A
push from the root carries every commit on the local `main`, the main loop's too. A red that comes
from the main loop's unfinished goal is told to it with `bun tools/looper/main.ts loop say --loop main
"<job>: <what failed and why>"` and passed over with `--past <job>`.

**A push run that goes red after a green `bun nv deep` names a gap, and the gap is fixed with the
bug.** For each red job, the session fixes the code and also makes `bun nv deep` catch it next time: a
step that was missing, a target that ran too short, a check the local step skipped. A red job that is
CI-only (macOS, aarch64) is fixed from its log alone, and the handoff says so. A failure a commit cannot
fix is rerun with `bun nv ci --rerun`: a runner that went away, a network error while downloading, a
GitHub outage. A test that fails the same way twice is not noise.

**A fix is checked by its local step, then by what it reaches.** `bun nv deep --only <job>` for each
job it fixes, then `bun nv affected --run`. Before each commit that touches Rust, `cargo fmt --all
--check` and `cargo clippy --all-targets -- -D warnings` are clean. The full `looper verify` is the
main loop's, and this goal never starts one.

**The loop stops itself once the push run is green.** The session writes the handoff, runs `bun
tools/looper/main.ts loop stop --loop side-release-green`, and writes the status line `CONTINUE CI
green on <sha>; a real Release of <sha> skips its test job`. This goal never dispatches, approves,
publishes or reruns a real `Release` or `Promote release images` run: each publishes something, and
that is a person's decision.

**Split.** `bun nv deep`'s red steps whose causes are independent go to writing subagents on the
session's model, one after another, each told its failure, the files to touch, the `bun nv deep
--only` step to run, and to commit with a pathspec. None of them pushes. A CI log longer than the tail
`--watch` prints is read by a read-only `reader` subagent, from `.agent-tmp/ci/`. The session keeps
the runs, the push and the handoff.

**Lanes.** This loop runs beside the main loop and every side loop, and shares the root with the main
loop only. Stage 1 touches `tools/nv/cmd/deep.ts` and `tools/nv/cmd/fuzz.ts`. Stage 2 touches the
workflows, `tools/nv/cmd/ci.ts`, `ci-changes.ts`, `release.ts` and the rules. Stage 2's table row
in `deep.ts` waits for stage 1.

## Stages

### 1. one command runs every CI job this machine can run

**Does:** builds `bun nv deep` and `bun nv fuzz --all`, as § *Standing decisions* says, then runs
`bun nv deep` once and records its time here.

**Split:** a read-only `reader` subagent lists each `ci.yml` job's `run:` lines and the tools each
needs (`file:line` and one line each). The session writes the code.

- [x] `tools/nv/cmd/deep.ts:1`: the job table, the runner, the report, `--failed` and `--only`.
- [x] `tools/nv/test/deep.test.ts:1`: every `ci.yml` job is in the table, as a step or as CI-only
  with its reason.
- [x] `tools/nv/cmd/fuzz.ts:1`: `--all`, and the list of every saved input at the end.
- [x] Each step that cannot run yet because a tool is missing in WSL or on Windows (cargo-deny, Miri,
  valgrind) gets its install line in `docs/agent/commands.md` § *Without looper* and in the step's
  error message.
- [ ] `bun nv selftest` is green, then one full `bun nv deep` run, with its time written into
  § *Standing decisions*. Its red steps are fixed in stage 3's first cycle.

### 2. one push run is the full suite, the dev build and the release's proof

**Does:** every run of `ci.yml` runs every job, `Dev build` merges into it, each push proves the
release's version step, and a green push run counts for `--tested`, as § *Standing decisions* says.

**Split:** a read-only `Explore` subagent lists every citation of `testing/ci-lanes`,
`testing/lane-table`, `testing/the-deep-lane`, `ci-changes`, `Dev build` and `dev-build.yml` in the
tree (`file:line`, one line each). The session writes the changes.

- [ ] `.github/workflows/ci.yml:1`: no `changes` job, no `full` input, no `if:` on a lane output.
  The build legs, the `dev` image job and the `--apply` check job join it.
- [ ] `.github/workflows/dev-build.yml:1` deleted, and `tools/nv/cmd/ci-changes.ts:1` with its
  tests.
- [ ] `.github/workflows/release.yml:224`: `test` calls `ci.yml` with no input. Its header and
  `docs/release.md` say a green push run is the proof.
- [ ] `tools/nv/cmd/release.ts:35`: `--tested` takes a green `ci.yml` run of any event on `main`.
  Its tests cover a push run, a red run and a run on another branch.
- [ ] `tools/nv/cmd/ci.ts:1`: no `Dev build` in its lists, and `--dispatch release` checks
  `--tested` on `main`'s tip. Its tests follow.
- [ ] The three rules rewritten or deleted, `bun nv rules --check` green, and the `deep.ts` rows
  for the build legs and the `--apply` check.
- [ ] Proved in stage 3's first push: its run runs every job, publishes the `dev` image, and `bun nv
  release --tested <sha>` prints `tested=true` for it.

### 3. the release cycle

Every session runs this cycle until the push run is green or the context reaches 120k:

1. **Run the local lane:** `bun nv deep` with `run_in_background` and a timeout of 7200000, then end
   your turn. Its exit code starts your next turn. After a fix, `bun nv deep --failed` runs only the
   steps that were red.
2. **Fix every red step in one pass,** as § *Standing decisions* says, one commit per fix with a
   pathspec. Then step 1 with `--failed`, until it exits 0.
3. **Push:** `bun nv ci --check`, then `bun nv ci --push`.
4. **Watch the push's run:** `bun nv ci --watch --next` with `run_in_background` and a timeout of
   7200000, then end your turn.
   - **Green:** stop the loop, as § *Standing decisions* says.
   - **A job failed:** let the run finish with `--past` for each job already seen, so one push carries
     every fix. Fix each, close each gap in `bun nv deep`, check each with `bun nv deep --only <job>`
     and `bun nv affected --run`, then step 3.
5. Once the context reaches 120k, write the handoff with what is fixed, what is pushed and which run
   is going, and the status line `CONTINUE <the fixes>, pushed in <sha>`, and stop.

## Handoff

### State

`bun nv deep` and `bun nv fuzz --all` are on `main` (`a27732169d`). The first full run stopped at the
two-hour background limit during `fuzz-smoke`, which has never run. `.agent-tmp/deep/report.json` has
the rest: 17 steps green, and these red:
- **`asan`, `tsan`, `miri-units`**: two `capability::tests` cases compared an unresolved path with
  the resolved one, and the WSL copy's `target` is a symlink. They compare resolved paths now
  (`9f69bf93e7`) and pass in WSL. Each step stopped at that crate, so the rest has not run.
- **`docker`**: `nvs-ext`'s build script writes `target/ext` into the read-only source mount. The step
  mounts a volume over `/src/target` now (committed), and has not run since.
- **Missing programs**, for the person who runs the loop to install (`docs/agent/commands.md`
  § *What `bun nv deep` needs installed*): wit-bindgen and wasi-sdk on Windows (`ext-templates`), Bun
  and Node in WSL (`proofs-linux`, `valgrind`), and Docker's WSL integration (`proofs-linux`,
  `test-linux`).

### Next

**Stage 1, last item.** Run the parts that have not run, each as its own background call: `bun nv deep --only docker`, `--only asan`, `--only tsan`,
`--only miri-units` and `--only fuzz-smoke` (about 90 minutes alone). Fix what goes red, and expect no
Rust test to go red from this session's change. Then tick the item and set stage 1 to done. Steps red
only for a missing program go into stage 3 as a decision for the person (`looper decision ask
--parked`), not as blockers. Then stage 2.

### Backlog

- `bun nv ci --watch --past "fuzz smoke / markdown"` does not cover the job
  `fuzz smoke / markdown, markdown_from_html`: the match takes the whole name, or a prefix ending
  at ` / ` or ` (`. Let a prefix ending at `,` match too, in `tools/nv/cmd/ci.ts`.
- `bun nv ci --check` does not run CI's `rustdoc links` step, so a broken doc link reaches CI. It
  takes about a minute once warm. Add it to `PRE_PUSH` in `tools/nv/cmd/ci.ts`.
- `crates/nvs-runtime/src/ctx/hooks.rs:603` says the sampler notices a crossing "up to one sweep
  late". Since `d59403277`, the sweep wakes at the earliest possible crossing.
- A CPU-time FATAL from a child task omits the time used, because it reads the child's
  `cpu_limit` of 0 and not the tree's. `crates/nvs-runtime/src/ctx/limits.rs:1101`.
- `publish_cpu_limit` (`crates/nvs-runtime/src/ctx/safepoint.rs:99`) does not wake the watchdog, so
  a limit lowered by `Core\Config::set` is first seen up to one 50 ms `DEFAULT_INTERVAL` late, on
  every platform. Wake it as `publish_wall_time` does.
