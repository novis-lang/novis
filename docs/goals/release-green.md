---
looper: 1
title: A Release dry run of main's tip is green
summary: "Never reached: each time a person starts its loop, it runs every CI job that can run on this machine, fixes every failure in one pass, pushes, cancels the CI runs the push started, dispatches one Release dry run, and fixes and repeats until that dry run is green. Then it stops its own loop."
context: [tools/nv/cmd/ci.ts, tools/nv/cmd/fuzz.ts, .github/workflows/ci.yml, .github/workflows/release.yml, looper.toml]
stages:
  - id: 1
    title: one command runs every CI job this machine can run
    status: open
    context: [tools/nv/cmd/ci.ts, tools/nv/cmd/fuzz.ts, tools/nv/cmd/linux-test.ts, .github/workflows/ci.yml]
  - id: 2
    title: a real release publishes what its green dry run built
    status: open
    context: [.github/workflows/release.yml, .github/workflows/build.yml, .github/workflows/image.yml, tools/nv/cmd/release.ts]
  - id: 3
    title: the release cycle
    status: open
    context: [tools/nv/cmd/ci.ts, .github/workflows/ci.yml, .github/workflows/release.yml]
checks: []
---

# A Release dry run of main's tip is green

## Why

Three `Release` runs in a row went red on one day, each on a bug that no push had tested: three
round trips that `fuzz smoke / markdown, markdown_from_html` found, Miri tests too large for Miri's
limit, and a CPU-time stop that wrote no sweep line on Windows. Each red cost a 75-minute run, and
each run found only the next bug. The push lane runs only what its diff reaches, and Miri, the
fuzzers and the full proofs run nightly and at release alone (`rule:testing/ci-lanes`). So the first
place they ran on new code was the release.

This goal moves that first run to this machine. Every CI job that can run here runs here first, every
failure it finds is fixed in one pass, and only then does a dry run go to GitHub. A dry run that is
still red after a green local run names a gap in the local run, and that gap is closed too.

A real `Release` skips its hour-long `test` job when the same commit already has a green dry run,
nightly or dispatched full `CI` run (`bun nv release --tested`). A green push run does not count.
Stage 2 makes it skip `build` too: the real run publishes the archives, the `.vsix` and the images
that the green dry run of the same commit and version built. So a green dry run is the whole release
but the writes, and the real run is the approval, the tag, the draft and the image push, as long as
nothing is pushed between the two.

This goal is never reached. A person starts its loop with `bun tools/looper/main.ts loop start
--loop side-release-green`, and the loop stops itself once the dry run is green.

## Standing decisions

**The loop works in the root, on `main`.** It is added with `bun tools/looper/main.ts loop add --side
release-green --in-root`, so it has no worktree, no branch and no landing. The main loop's sessions
edit the same tree at the same time, so every rule for two sessions in one tree holds: commit with a
pathspec of your own files, never run a tree-wide git command (`stash`, `checkout -- .`, `reset`,
`add -A`, `clean`), and leave every file you did not write as it is. The claims guard refuses a file
another session holds.

**`bun nv deep` is the local lane: every `ci.yml` job this machine can run, in one call.** It is new
in stage 1, in `tools/nv/cmd/deep.ts`. Each `ci.yml` job maps to one local step, which runs the same
`bun nv` command or `cargo` line the job runs. A job with no local step is listed as CI-only with its
reason (macOS, aarch64). A test fails when `ci.yml` has a job the table lacks, so a new CI job is never
skipped unseen. The map, with each step stage 1 confirms against the job's `run:` lines:

| `ci.yml` job | Local step |
|---|---|
| `changes` | none: it picks lanes, and the local lane runs all of them |
| `test` (windows-x86_64) | `bun nv verify --full` |
| `test` (linux-x86_64) | the whole suite in WSL: `bun nv linux-test` with no filter, extended in stage 1 if it needs one |
| `test` (macos-aarch64, linux-aarch64) | CI-only: no such machine here |
| `cost-guards` | the job's release-mode guard runs, as the job runs them |
| `proofs` | `bun nv proofs --gate` on Windows, and the Linux leg in WSL |
| `database` | `bun nv db-matrix --all` |
| `browser`, `extension-sandbox`, `extension`, `extension-host` | the job's own commands, read from `ci.yml` |
| `ext-templates` | `bun nv ext-templates` |
| `webp-lib` | `bun nv webp-lib --check` |
| `docker` | the job's image build, with Docker Desktop's Linux engine |
| `lint`, `attribution`, `docs` | `bun nv ci --check`, which runs these checks already, and `bun nv verify --doc` |
| `supply-chain` | `cargo deny check`, the version the job pins |
| `reference` | `bun nv reference` |
| `unsafe-audit`, `flood` | the job's own commands, read from `ci.yml` |
| `fuzz-smoke` | `bun nv fuzz <target> --seconds <n> --all` for each target the job fuzzes, in WSL |
| `miri`, `miri-units` | the jobs' `cargo miri` lines in WSL, on the toolchain the job pins |
| `valgrind` | `bun nv valgrind` in WSL |
| `asan`, `tsan` | the jobs' sanitizer lines in WSL |

**`bun nv deep` runs every step and stops at none.** It prints one line per step as it ends, and at the
end each red step with its command and the tail of its log. It writes the whole report to
`.agent-tmp/deep/` and exits 1 when a step is red. `--failed` runs again only the steps the last report
names red. `--only <job>` runs one job's step. One step runs at a time, so a timing guard never shares
the machine with a fuzzer or Miri. What it costs: about one to two hours of this machine per full run,
which stage 1 measures and records here. That is less than one red 75-minute dry run plus its fix and
its second run.

**`bun nv fuzz --all` keeps fuzzing after a crash.** It runs libFuzzer's fork mode with
`-ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1`, and at the end lists every input it saved,
each with its base64. So one run finds every crash it can, and all of them are fixed in one pass.
Each target runs `120` seconds on the fork mode's jobs, about three times the work of the CI job's
300 seconds on one core. A target whose `fuzz/fuzz_targets/<target>.rs` is younger than 14 days runs
`1800` seconds, because a young target finds a new bug every few minutes. `markdown_comrak` stays out,
as it does in CI.

**`bun nv ci --cancel` cancels every `CI` and `Dev build` run going or queued on `main`.** It is new
in stage 1. A dry run's `verify` job is a full `ci.yml` run, so a push run of the same commit proves
nothing more and takes runners from the dry run, macOS ones first. It never cancels a `Website` run,
which deploys the site, or a `Release` or `Promote release images` run, which publish. A cancelled
push run leaves `:dev` one push behind until the next push.

**The session pushes, cancels and dispatches, in that order, and only after a green local run.** `bun
nv ci --check`, then `bun nv ci --push`, then `bun nv ci --cancel`, then `bun nv ci --dispatch
dry-run`, then `bun nv ci --watch --release`. Never `git push`, never a force push, and never a push
of another branch. A push from the root carries every commit on the local `main`, the main loop's
too. A red that comes from the main loop's unfinished goal is told to it with `bun
tools/looper/main.ts loop say --loop main "<job>: <what failed and why>"` and passed over with
`--past <job>`.

**A dry run that goes red after a green `bun nv deep` names a gap, and the gap is fixed with the bug.**
For each red job, the session fixes the code and also makes `bun nv deep` catch it next time: a step
that was missing, a target that ran too short, a check the local step skipped. A red job that is
CI-only (macOS, aarch64) is fixed from its log alone, and the handoff says so. A failure a commit cannot
fix is rerun with `bun nv ci --rerun`: a runner that went away, a network error while downloading, a
GitHub outage. A test that fails the same way twice is not noise.

**A fix is checked by its local step, then by what it reaches.** `bun nv deep --only <job>` for each
job it fixes, then `bun nv affected --run`. Before each commit that touches Rust, `cargo fmt --all
--check` and `cargo clippy --all-targets -- -D warnings` are clean. The full `looper verify` is the
main loop's, and this goal never starts one.

**The loop stops itself once the dry run is green.** The session writes the handoff, runs `bun
tools/looper/main.ts loop stop --loop side-release-green`, and writes the status line `CONTINUE dry
run green on <sha>; a real Release of <sha> skips its test and build jobs`. This goal never dispatches,
approves, publishes or reruns a real `Release` or `Promote release images` run: each publishes
something, and that is a person's decision.

**A real `Release` reuses the build of a green dry run of the same commit and version.** `plan` looks
it up with `bun nv release --built "$SHA" "$VERSION" --github-output`, which is new in stage 2 and
sets `built_run`. A run counts when it is a `release.yml` dry run, dispatched on `main`, at
`github.sha`, green, with its `build` job green and its artifacts not expired. Its version is read
from a `release-plan.json` artifact that every run's `plan` now uploads, with the version and the
commit. When `built_run` is set, `build` is skipped, `publish` downloads `dist-*` from that run
(`run-id` and the run's token, with `actions: read`), and `docker` passes it as `binaries-run`,
which `image.yml` takes already. With no such run, the real run builds as it does now.
- **Security:** before anything is written, `publish` checks each archive and the `.vsix` with `gh
  attestation verify`, and each provenance must name `release.yml` and `github.sha`. The flags come
  from the installed `gh`'s help. The `release` environment's approval stays the gate. The
  provenance names the dry run that built the bytes, which is true.
- **The version:** a dry run builds with the version its `plan` computed from its own `bump`
  input. A real run with another `bump`, or after another tag, has another version, so it builds.
- **Expiry:** `build.yml` keeps a versioned build's artifacts 7 days (`retention-days`). A real run
  later than that builds again.
- **What it costs:** nothing on the dry run, which builds all of it already. The real run loses
  `build`'s macOS and Windows legs, about 20 minutes, and keeps `docker`'s packaging, which compiles
  nothing.

**Split.** `bun nv deep`'s red steps whose causes are independent go to writing subagents on the
session's model, one after another, each told its failure, the files to touch, the `bun nv deep
--only` step to run, and to commit with a pathspec. None of them pushes. A CI log longer than the tail
`--watch` prints is read by a read-only `reader` subagent, from `.agent-tmp/ci/`. The session keeps
the runs, the push, the dispatch and the handoff.

**Lanes.** This loop runs beside the main loop and every side loop, and shares the root with the main
loop only. Stage 1 touches `tools/nv/` alone, and stage 2 the release workflows and
`tools/nv/cmd/release.ts`.

## Stages

### 1. one command runs every CI job this machine can run

**Does:** builds `bun nv deep`, `bun nv fuzz --all` and `bun nv ci --cancel`, as § *Standing
decisions* says, then runs `bun nv deep` once and records its time here.

**Split:** a read-only `reader` subagent lists each `ci.yml` job's `run:` lines and the tools each
needs (`file:line` and one line each). The session writes the code.

- [ ] `tools/nv/cmd/deep.ts:1`: the job table, the runner, the report, `--failed` and `--only`.
- [ ] `tools/nv/test/deep.test.ts:1`: every `ci.yml` job is in the table, as a step or as CI-only
  with its reason.
- [ ] `tools/nv/cmd/fuzz.ts:1`: `--all`, and the list of every saved input at the end.
- [ ] `tools/nv/cmd/ci.ts:1`: `--cancel`, and its test in `tools/nv/test/ci.test.ts`.
- [ ] Each step that cannot run yet because a tool is missing in WSL or on Windows (cargo-deny, Miri,
  valgrind) gets its install line in `docs/agent/commands.md` § *Without looper* and in the step's
  error message.
- [ ] `bun nv selftest` is green, then one full `bun nv deep` run, with its time written into
  § *Standing decisions*. Its red steps are stage 3's first cycle.

### 2. a real release publishes what its green dry run built

**Does:** the real run reuses the green dry run's build, as § *Standing decisions* says.

- [ ] `.github/workflows/release.yml:124`: `plan` uploads `release-plan.json` and sets `built_run`
  from `bun nv release --built`.
- [ ] `.github/workflows/release.yml:247`: `build` runs only without `built_run`. `publish` reads
  a skipped `build` as passed when `built_run` is set, downloads from that run, and verifies each
  attestation before its first write. `docker` passes `binaries-run`.
- [ ] `tools/nv/cmd/release.ts:1`: `--built`, and its tests in `tools/nv/test/`: a red run, a run
  whose `build` was skipped, another commit, another version and expired artifacts each give no
  `built_run`.
- [ ] A dry run on a commit that already has a green dry run takes the same path: it skips `build`,
  downloads, verifies and runs `docker`'s smoke test. Prove the stage with two `bun nv ci --dispatch
  dry-run` runs on one commit, the second one green with `build` skipped. No real run is started.

### 3. the release cycle

Every session runs this cycle until the dry run is green or the context reaches 120k:

1. **Run the local lane:** `bun nv deep` with `run_in_background` and a timeout of 7200000, then end
   your turn. Its exit code starts your next turn. After a fix, `bun nv deep --failed` runs only the
   steps that were red.
2. **Fix every red step in one pass,** as § *Standing decisions* says, one commit per fix with a
   pathspec. Then step 1 with `--failed`, until it exits 0.
3. **Push, cancel, dispatch:** `bun nv ci --check`, `bun nv ci --push`, `bun nv ci --cancel`, `bun nv
   ci --dispatch dry-run`. Do not dispatch while a `release.yml` run is going: `bun nv ci
   --release-status` says so, and the new run would wait behind it.
4. **Watch the dry run:** `bun nv ci --watch --release` with `run_in_background` and a timeout of
   7200000, then end your turn.
   - **Green:** stop the loop, as § *Standing decisions* says.
   - **A job failed:** let the run finish with `--past` for each job already seen, so one push carries
     every fix. Fix each, close each gap in `bun nv deep`, check each with `bun nv deep --only <job>`
     and `bun nv affected --run`, then step 3.
5. Once the context reaches 120k, write the handoff with what is fixed, what is pushed and which run
   is going, and the status line `CONTINUE <the fixes>, pushed in <sha>`, and stop.

## Handoff

### State

The goal replaced `ci-green`, whose loop watched every push run and fixed each red one at a time.
Its last fixes are on `main`: `2d1f891fd5`, `1b237c54e2` and `6556919af6` for
`markdown_from_html`, the Miri test sizes in `3986e9d237`, `3164c81133` and `0e0d7e58a0`, and the
sweep line of a CPU-time stop in `f64caa415d`. No dry run has run on `f64caa415d` yet. The nightly
`CI` run `42468563179` was red on Miri, on `markdown_from_html`, and on downloads (Redis on Windows,
`cargo-deny-action`, `wasi-sdk`).

### Next

**Stage 1.** Build `bun nv deep`, `bun nv fuzz --all` and `bun nv ci --cancel`, run `bun nv deep`
once. Then stage 2, then stage 3 on `bun nv deep`'s red steps.

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
