---
looper: 1
title: "GitHub CI stays green on main"
summary: "Never reached: a loop in the project root watches every CI run on main, fixes each failure the moment a job reports it, commits, pushes and watches again, for as long as main changes."
context: [tools/nv/cmd/ci.ts, .github/workflows/ci.yml, .github/workflows/pages.yml, looper.toml]
stages:
  - id: 1
    title: "the newest CI and Website runs on main are green"
    status: open
    context: [tools/nv/cmd/ci.ts, .github/workflows/ci.yml, .github/workflows/pages.yml]
---

# GitHub CI stays green on main

## Why

The loop commits to `main` all day, and CI is the only place the whole matrix runs: three operating
systems, Miri, the fuzzers, the release guards, the proofs and the website. A failure found an hour
after its push costs a cold cache and a stack of commits to bisect. A failure found the minute its job
reports it costs one fix.

This goal is never reached. It runs as its own loop in the project root, beside the main loop, and
nothing retires it. A person stops it with `looper loop stop --loop side-ci-green`.

## Standing decisions

**The loop works in the root, on `main`.** It was added with `looper loop add --side ci-green
--in-root`, so it has no worktree, no branch and no landing. The main loop's sessions edit the same
tree at the same time, so every rule for two sessions in one tree holds: commit with a pathspec of your
own files, never run a tree-wide git command (`stash`, `checkout -- .`, `reset`, `add -A`, `clean`), and
leave every file you did not write as it is. The claims guard refuses a file another session holds.

**`main` is pushed at three moments, always through `bun nv ci --push`.** Once when this goal's loop
starts for the first time, at every goal end through `looper.toml`'s `after_retire` hook, and after
each fix this goal commits. Never `git push`, never a force push, and never a push of another branch.
A push sends everything on the local `main`, the main loop's finished slices with it.

**`bun nv ci` is the only way the session talks to GitHub.** `--push` pushes, and the push starts the
CI and Website runs. Never start or cancel a run by hand.
`--watch` waits for the first failed job and prints its step and the end of its log. `--status` lists
every failed job of the newest runs. `--rerun` reruns the failed jobs. Their module doc is
`tools/nv/cmd/ci.ts`.

**CI is this goal's gate, not `looper verify`.** A fix is checked locally by the one thing that failed:
`cargo build`, then the failing test by name (`cargo test --test <name> <filter>`), or
`bun nv verify --fast -p <crate>`, or the `bun nv` step the job runs. Before each commit that touches
Rust, `cargo fmt --all --check` and `cargo clippy --all-targets` must be clean. The full `looper
verify` is the main loop's, and this goal never starts one.

**A failure a commit cannot fix is rerun, not fixed.** A runner that went away, a network error while
downloading, a GitHub outage (githubstatus.com), or a timing guard that failed with no code change
(`docs/agent/playbook.md`'s measuring traps): `bun nv ci --rerun`, then watch again. A test that
fails the same way twice is not noise.

**A red that the main loop's open work owns is handed to it.** When the failing file is claimed by the
main loop's session, or the failure is a feature proof or a perf record the live goal's open stage is
still writing, tell the main loop with `bun tools/looper/main.ts loop say --loop main "<job>: <what
failed and why>"` and watch again. Everything else that is red is this goal's to fix, whoever wrote it.

**Nothing here is ever ticked.** The goal has one stage that stays open, and no checkpoints. The
handoff's *State* is the only record.

**Split.** Each failed job whose cause is independent of the others may go to its own writing
subagent, one after another, each told its failure, the files to touch, the local check to run and to
commit with a pathspec. Read a CI log through a read-only subagent when it is longer than the tail
`--watch` prints. The whole log of each failed job is under `.agent-tmp/ci/`.

**Lanes.** This loop runs beside the main loop and every side loop, and shares the root with the main
loop only.

## Stages

### 1. the newest CI and Website runs on main are green

Every session runs this cycle until CI is green or the context reaches 120k:

1. **Push once, at the very first session only:** `bun nv ci --push`. The handoff's *State* says
   whether that push happened. Every later session skips this step.
2. **Watch:** `bun nv ci --watch` with `run_in_background` and a timeout of 7200000, then end your turn.
   Its exit code starts your next turn. Pass `--past <job>` for each red already handed to the main
   loop, so the watch goes on for the other jobs.
   - **0, green:** write the handoff and the status line `CONTINUE CI green on <sha>`, and stop. The
     next session's watch waits for the next push.
   - **1, a job failed:** go to step 3.
   - **2, `gh` failed twenty times in a row:** check githubstatus.com and `gh auth status`, then
     watch again. An outage that lasts is reported in the status line with `CONTINUE`.
   - **3, nothing failed in 110 minutes:** watch again in the same session.
   - **4, only the `--past` jobs failed:** write the handoff and the status line `CONTINUE CI green
     on <sha> but for <job>`, and stop.
3. **Fix:** `bun nv ci --status` lists every job that failed so far. Find each cause in the code,
   fix it, check it locally as § *Standing decisions* says, and commit each fix on its own with a
   pathspec.
4. **Push:** `bun nv ci --push`. The push starts new runs. Go to step 2.

## Handoff

### State

`origin/main` is at `dbef3c67b`. Its CI run,
https://github.com/novis-lang/novis/actions/runs/39593379567, and its Website run,
https://github.com/novis-lang/novis/actions/runs/39593379682, are green. The nightly deep lane on
`afbb1edc2` had turned the `proofs` stale guard red on every platform. `5d6d69097` restamped the four syntax
pages and eleven question sets the class-property wording left stale. Its push then turned `docs` red,
because `078f54e91` changed rule fragments without rendering their chapters. `dbef3c67b` committed the
rendered `docs/rules/errors.md`, `docs/rules/types.md` and `docs/ground-rules.md`. Both fixes were checked
in a scratch worktree at `HEAD`, so the main loop's uncommitted edits stayed out. The push lane on
`dbef3c67b` ran few jobs, so the next nightly deep lane is the first full check of this tree. No red is
handed to the main loop, so no watch passes `--past`.

### Next

**Stage 1, the cycle.** Skip step 1, because the first push happened.

- [ ] Watch with `bun nv ci --watch`, which waits for the next push. `tools/nv/cmd/ci.ts:224`
- [ ] Fix and push each red job. `.github/workflows/ci.yml:1`
  A stale guard red is fixed by `bun nv site --stale` and `bun nv site --questions --owed`, then a
  reread and a `--stamp`. Under Git Bash, a URL argument needs `MSYS_NO_PATHCONV=1`. Check a site or
  render red against committed `HEAD` in a scratch worktree with `--nvs E:/mwl/target/debug/nvs.exe`,
  because the root holds the main loop's uncommitted edits. A `git cherry-pick` into the root fails on
  its staged changes, so apply the diff with `git apply` and commit by pathspec.

### Backlog

Nothing waits.

