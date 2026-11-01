---
looper: 1
title: GitHub CI stays green on main
summary: "Never reached: a loop in the project root watches the CI and Website runs of every push to main, fixes each failure the moment a job reports it, commits, pushes, and watches again until the runs are green, then waits for the next push."
context: [tools/nv/cmd/ci.ts, .github/workflows/ci.yml, .github/workflows/pages.yml, looper.toml]
stages:
  - id: 1
    title: the newest CI and Website runs on main are green
    status: open
    context: [tools/nv/cmd/ci.ts, .github/workflows/ci.yml, .github/workflows/pages.yml]
checks: []
---

# GitHub CI stays green on main

## Why

The loop commits to `main` all day, and CI is the only place the whole matrix runs: three operating
systems, Miri, the fuzzers, the cost guards, the proofs and the website. A failure found an hour after
its push costs a cold cache and a stack of commits to bisect. A failure found the minute its job
reports it costs one fix, and a fix pushed at once is checked by CI within the hour, not at the next
goal end.

This goal is never reached. It runs as its own loop in the project root, beside the main loop, and
nothing retires it. A person stops it with `bun tools/looper/main.ts loop stop --loop side-ci-green`.

## Standing decisions

**The loop works in the root, on `main`.** It is added with `bun tools/looper/main.ts loop add --side
ci-green --in-root`, so it has no worktree, no branch and no landing. The main loop's sessions edit
the same tree at the same time, so every rule for two sessions in one tree holds: commit with a
pathspec of your own files, never run a tree-wide git command (`stash`, `checkout -- .`, `reset`,
`add -A`, `clean`), and leave every file you did not write as it is. The claims guard refuses a file
another session holds.

**This loop pushes its own fixes, at once.** `looper.toml`'s `after_retire` hook still pushes `main`
at every goal end, and that push is the one this loop usually watches. Once this loop has committed
its fixes for a red run, it runs `bun nv ci --push` itself and watches the runs that push starts,
until they are green. Never `git push`, never a force push, and never a push of another branch.

What that costs: a push from the root carries every commit on the local `main`, so it also carries
the main loop's commits of a goal whose gate stage has not run. CI may then go red on that goal's
unfinished work. Such a red is the main loop's, as the decision below says, and is passed over with
`--past`. The other way, holding each fix until the next goal end, leaves CI red for a whole goal and
lets the next goal's reds pile onto it. The user chose the push.

**A refused push is fixed before anything else.** `bun nv ci --push` runs its pre-push checks on the
working tree first, and a red one refuses the push. `--watch` and `--status` then print the red checks
and exit 1. A red check in a file this loop or a committed change owns is fixed, committed, and
checked with `bun nv ci --check` until it is green; then push again. A red check that only the main
loop's uncommitted edits cause (`git status` shows the file changed, and `git diff` shows work this
loop did not write) is not fixed here: write the handoff, and the next session pushes once the main
loop has committed that file.

**`bun nv ci` is the only way the session talks to GitHub.** `--push` pushes, and the push starts the
CI and Website runs and cancels the runs it replaces. Never start or cancel a run by hand.
`--watch` waits for the first failed job and prints its step and the end of its log. `--status` lists
every failed job of the newest runs. `--rerun` reruns the failed jobs. Their module doc is
`tools/nv/cmd/ci.ts`.

**CI is this goal's gate, not `looper verify`.** A fix is checked locally by the one thing that failed:
`cargo build`, then the failing test by name (`cargo test --test <name> <filter>`), or
`bun nv verify --fast -p <crate>`, or the `bun nv` step the job runs. A path, socket or process fix
for Linux or macOS runs its test in WSL with `bun nv linux-test`. Before each commit that touches
Rust, `cargo fmt --all --check` and `cargo clippy --all-targets -- -D warnings` must be clean. The
full `looper verify` is the main loop's, and this goal never starts one.

**A failure a commit cannot fix is rerun, not fixed.** A runner that went away, a network error while
downloading, a GitHub outage (githubstatus.com), or a timing guard that failed with no code change
(`docs/agent/playbook.md`'s measuring traps): `bun nv ci --rerun`, then watch again. A test that
fails the same way twice is not noise.

**A red that the main loop's open work owns is handed to it.** When the failing file is claimed by the
main loop's session, or the failure is in code, a feature proof or a perf record the live goal's open
stages are still writing, tell the main loop with `bun tools/looper/main.ts loop say --loop main
"<job>: <what failed and why>"` and watch again with `--past <job>`. Everything else that is red is
this goal's to fix, whoever wrote it. Before fixing a red, run `git log origin/main..HEAD`: the main
loop may have committed the fix already, and then the push is the fix.

**Nothing here is ever ticked.** The goal has one stage that stays open, and no checkpoints. The
handoff's *State* is the only record, and each session overwrites it.

**Split.** Each failed job whose cause is independent of the others goes to its own writing subagent,
on the session's model, one after another. Each is told its failure, the files to touch, the local
check to run, and to commit with a pathspec; it never pushes. A CI log longer than the tail `--watch`
prints is read by a read-only `reader` subagent. The whole log of each failed job is under
`.agent-tmp/ci/`. The session keeps only the watch, the push and the handoff.

**Lanes.** This loop runs beside the main loop and every side loop, and shares the root with the main
loop only.

## Stages

### 1. the newest CI and Website runs on main are green

Every session runs this cycle until CI is green or the context reaches 120k:

1. **Push what is owed.** If `bun nv ci --status` names a refused push, or this goal has fixes
   committed that `origin/main` lacks, run `bun nv ci --push`, as § *Standing decisions* says.
2. **Watch:** `bun nv ci --watch` with `run_in_background` and a timeout of 7200000, then end your turn.
   Its exit code starts your next turn. Pass `--past <job>` for each red already handed to the main
   loop, and `--next` when the newest runs already ended and were seen, so the watch waits for the
   next push.
   - **0, green:** write the handoff and the status line `CONTINUE CI green on <sha>`, and stop. The
     next session's watch, with `--next`, waits for the next push.
   - **1, a job failed, or a push was refused:** go to step 3.
   - **2, `gh` failed twenty times in a row:** check githubstatus.com and `gh auth status`, then
     watch again. An outage that lasts is reported in the status line with `CONTINUE`.
   - **3, nothing failed in 110 minutes:** watch again in the same session.
   - **4, only the `--past` jobs failed:** write the handoff and the status line `CONTINUE CI green
     on <sha> but for <job>`, and stop.
3. **Fix:** `bun nv ci --status` lists a refused push's red checks and every job that failed so far.
   A refused push goes first. Find each cause in the code, fix it, check it locally as § *Standing
   decisions* says, and commit each fix on its own with a pathspec. A run still going may fail more
   jobs: watch it to its end with `--past` for the jobs already fixed before you push, so one push
   carries every fix for that run.
4. **Push and watch again:** `bun nv ci --push`, then step 2. Once the context reaches 120k, write
   the handoff with what was pushed and what is still red, and the status line `CONTINUE <the
   fixes>, pushed in <sha>`, and stop.

## Handoff

### State

`origin/main` is at `a6ef88c92`. It carries the four fixes of the last run. Website 41283592505 on
it is green. CI 41283592496 was still going when this session ended, and two jobs had failed:

- `the chapter examples run and the website matches the binary`: `nv reference` failed on
  `10-cli.md:1201`, "an `output` block with no program before it". The local commit `741bc5748`
  fixes it, and `bun nv ci --check`'s `chapter examples` check is green on it. It is not pushed.
- `proofs / macos-aarch64`: `tests/hostile/core/Compress/decompress/02-bombs-until-the-cpu-limit.nvs`
  used 371 ms against a CPU limit of 250 ms, 121 ms past the limit. The bound is the limit plus
  100 ms. The case passed on the push before. The diff between the two pushes touches only the
  wall-time claim and the `onLimit` handler's slice, and this attack has neither. So it is read as
  a timing failure on a loaded runner, and the next push reruns it. The decoder polls the deadline
  every 256 KiB (`crates/nvs-stdlib/src/compress.rs:819`), and the sweep runs every 50 ms
  (`crates/nvs-host/src/watchdog.rs:226`).

`bun nv ci --check` was green at the start of this session, and 12 commits wait to be pushed.
`bun nv ci --push` then failed its `build` check on `crates/nvs-runtime/src/ctx/held.rs`. The main
loop was editing that file and had not committed it (`started_scripts` changed type, and
`filed_at` still takes `&[(u64, T)]`). While a push is refused, `--watch` prints the refusal and
exits 1. So it cannot watch the run in progress.

### Next

**Stage 1, the cycle.**

- [ ] Push once `crates/nvs-runtime/src/ctx/held.rs:47` is committed. `tools/nv/cmd/ci.ts:1`
  Check it with `git status --short -- crates/nvs-runtime/src/ctx/held.rs`. If the file is still
  modified, hand off again. Once it is committed, run `bun nv ci --push`, then `bun nv ci --watch`.
  A push refused for another reason in a committed file is this loop's to fix.
- [ ] Expect both reds green on that push. `crates/nvs-stdlib/src/compress.rs:950`
  If the Compress CPU attack goes over the bound a second time, it is not noise. Look at what runs
  between the crossing and the stop: the 16 MiB `Vec` that `read_to_end` grows there, and the
  sweep's own lateness.

### Backlog

- A CPU-time FATAL from a child task omits the time used, because it reads the child's
  `cpu_limit` of 0 and not the tree's. `crates/nvs-runtime/src/ctx/limits.rs:1101`. It waits
  because no job is red on it.
