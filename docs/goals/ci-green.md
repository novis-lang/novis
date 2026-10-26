---
looper: 1
title: GitHub CI stays green on main
summary: "Never reached: a loop in the project root watches the CI run of every goal end's push to main, fixes each failure the moment a job reports it, commits, and watches again; the next goal end pushes the fixes."
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

**`main` is pushed only at a goal's end, by `looper.toml`'s `after_retire` hook** (the user's call).
A goal is verified once, in its last stage, so the local `main` carries unverified commits while a
goal runs, and CI never sees them. This goal never pushes: it commits each fix, and the fix goes out
with the next goal end's push. Never `git push`, never a force push, and never a push of another
branch. The one exception is a **refused push**: `bun nv ci --push` runs its pre-push checks first,
and a red one refuses the goal end's push. `--watch` and `--status` then print the red checks and
exit 1. Fix each, commit, run `bun nv ci --check` until it is green, and then run `bun nv ci --push`
yourself. That push is the goal end's, a little late.

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

1. **Never push**, except to send a refused push once its checks are green. The next goal end
   pushes, and its push starts the runs the watch waits for.
2. **Watch:** `bun nv ci --watch` with `run_in_background` and a timeout of 7200000, then end your turn.
   Its exit code starts your next turn. Pass `--past <job>` for each red already handed to the main
   loop, so the watch goes on for the other jobs.
   - **0, green:** write the handoff and the status line `CONTINUE CI green on <sha>`, and stop. The
     next session's watch waits for the next push.
   - **1, a job failed, or the goal end's push was refused:** go to step 3.
   - **2, `gh` failed twenty times in a row:** check githubstatus.com and `gh auth status`, then
     watch again. An outage that lasts is reported in the status line with `CONTINUE`.
   - **3, nothing failed in 110 minutes:** watch again in the same session.
   - **4, only the `--past` jobs failed:** write the handoff and the status line `CONTINUE CI green
     on <sha> but for <job>`, and stop.
3. **Fix:** `bun nv ci --status` lists a refused push's red checks and every job that failed so far.
   A refused push goes first, as § *Standing decisions* says. Find each cause in the code,
   fix it, check it locally as § *Standing decisions* says, and commit each fix on its own with a
   pathspec.
4. **Hand off:** write the handoff with the fixes committed and not yet pushed, and the status line
   `CONTINUE <the fixes>, out with the next goal end`, and stop. The next session's watch waits for
   that push.

## Handoff

### State

`origin/main` is at `d6f131752`, `debugger-adapter`'s goal end. Its Website run 40249167966 is
green. Its CI run 40249168095 ended red in three jobs, all fixed and not pushed:

- `supply chain`: crypto-bigint 0.7.6 was yanked. The main loop's `97ccf66c2` takes 0.7.9.
- `test / macos-aarch64 (tests)`: `calls_one_after_another_use_one_thread_and_allocate_the_same`
  counted batches `[1500, 1500, 1499, 1500]`. The main loop's `a60965625` locks the answer slot on
  the core before it submits, since a pthread mutex allocates on its first lock.
- `test / windows-x86_64 (tests)`: `two_counted_runs_with_a_collection_print_the_same_line` printed
  `bytes=` 63 apart for its two cold runs. Under the test's 2M ceiling the project folder's path
  length moves the collections. Measured here: with no ceiling the counts match for every path, and
  at 2M a folder name of 51 bytes against 50 changed `allocations=` too. `9c8b0fe16` runs all three
  runs in one folder, each cold run with its own data folder. Whether the macOS red of
  `94245fd46` had the same cause is not known.

Every other job is green, `five-driver matrix` and the Windows idle-stack test among them, so
`947d5b7b3` and `a78076b26` hold. `cargo fmt --all --check` passed on CI with the dap files in.

The push lane skipped the Miri and overload jobs, so the fixes for runs 40158642410 and
40174915924 are not checked yet. They run in the nightly deep lane.

No push has gone out since. Three `--watch --next` runs in a row exited 3, about six hours with no
new run, while `debug-service` stayed open with 28 unpushed commits. At `cfee6562b`,
`bun nv proofs --gate --limit 0` owes perf figures for `request.rs`, `uri.rs`, `queue.rs`,
`debug.rs`, `script.rs` and `json.rs`, the four `debug.*` directive proofs, and
`Core\Server::traceId`'s `about.md`, which has 206 words. `bun nv site --check questions` names
`/reference/core/server/traceid/` as stale. All of these come from `debug-service`'s commits or the
root's open edits, so they were handed to the main loop with `loop say`. Its goal end's pre-push
check refuses the push until they are paid. If `--watch` reports a refused push, check first whether
the main loop is still paying them.

`.agent-tmp/wt-perf`, `wt-csv` and `wt-render` are stray folders with a `.looper` directory that
Windows keeps while looper runs. `wt-csv` has a `target/` warm at `f6883f463`. Delete all three
when looper is stopped.

### Next

**Stage 1, the cycle.** Never push: `main` goes out at the next goal end.

- [ ] Watch the next push with `bun nv ci --watch --next`. `tools/nv/cmd/ci.ts:265`
  Run 40249168095 has ended, so `--next` passes over it. The next goal end's push carries
  `97ccf66c2`, `a60965625` and `9c8b0fe16`. Watch with no `--past`, and without a pipe into `tail`, which
  hides the exit code. A watch without `--next` exits 1 at once on the newest red run, even an
  old nightly one. If `proofs` fails on perf figures of `math.rs`, `test.rs`, `request.rs` or
  `json.rs`, bench-counters' gate did not record them: record them as below. If a `Core\Process` example fails on another program,
  name that program in its folder's `nvs.toml` too. If the one-frame test fails again, the
  message names the frames and the failures. More than half the failed frames arriving, on two runs in a
  row, is a real regression in `crates/nvs-server/src/socket.rs:send`, not a kernel buffer:
  `crates/nvs-server/src/serve.rs:sends_to_a_peer_that_stopped_reading_queue_one_frame_at_the_most`.
  `bun nv ci --rerun` reruns every failed job of the run, so a rerun for one flaky job reruns
  the known reds too. An exit of 3 with a run still going means the push came late: watch again.
  Before you watch, run `bun nv site --check stale questions` and `bun nv proofs --gate --limit 0`
  at `HEAD`. Use `--gate`, never `--owed`: `--owed` printed nothing while CI owed 43 figures.
  Record each stale perf figure from a committed change. A stale figure or page from a file the main
  loop has not committed yet is its open work. A record of `0` ns/op at 256 iterations is normal:
  the recorder writes the median beside it.
  A reference card rewrite stales three things in turn: the perf figures of its file, the render of
  `core.json`, and, once that is rendered, every question and page stamp built on the card. The
  questions reread splits well: a scratch script writes `bun nv site --questions --owed <area>
  --limit 1000` into batches of 45 pages with each page's current questions, and read-only `Explore`
  subagents on `sonnet` report only the pages to change. `--limit 0` there means zero pages.
- [ ] If the counted-run test fails again, compare the cold and warm lines. `crates/nvs-cli/tests/check_command.rs:333`
  `9c8b0fe16` took the folder path out of it. A red on "a warm run" is a different cause: the count
  window opens at `crates/nvs-cli/src/main.rs:2485`, and `0cc897b74` fixed state made on first use
  there once. A red on "a second cold run" means another per-run input moves the collections.
- [ ] If the LDAP login attack fails again, read the `proofs / linux-x86_64` log. `tests/hostile/core/Ldap-Connection/authenticate/01-guess-logins-and-passwords.nvs:1`
  It passed on `88fb690a3`. A second `whoami: Timeout` is not noise. Compare the socket deadline with the slice yield
  commits `13004e4f9` and `2b292f862`.
- [ ] If `five-driver matrix` fails again, read SQL Server's own log. `tools/nv/cmd/db-matrix.ts:367`
  The step now prints `db-matrix: the last 60 lines of `mssql`'s log:` above its error. Fix what
  that log names, in `tests/db/compose.yaml`'s `mssql` service. When every service is `Healthy` and
  a leg names a test, the red is that test's, as on `49bf05736`. Check a fix with `bun nv db-matrix`.
  Docker Desktop runs here.
- [ ] Fix and commit each other red job. `.github/workflows/ci.yml:1`
  A render red needs a binary built at `HEAD`: the root's `target/debug/nvs.exe` carries the main
  loop's uncommitted edits. Build one with `cargo build --bin nvs` in the scratch worktree.
  It takes about 15 minutes cold. Set `CARGO_TARGET_DIR=E:/mwl/.agent-tmp/wt-csv/target`, which is
  warm, and it takes a few minutes. Then render with `NVS_BIN` pointing at the `nvs.exe` it built.
  A stale guard red is fixed by `bun nv site --stale` and `bun nv site --questions --owed`, then a
  reread and a `--stamp`. Under Git Bash, a URL argument needs `MSYS_NO_PATHCONV=1`. Check a site or
  render red against committed `HEAD` in a scratch worktree. `bun nv render` takes no `--nvs`, so
  run it with `NVS_BIN=E:/mwl/target/debug/nvs.exe`. The worktree is needed because the root holds
  the main loop's uncommitted edits. A `git cherry-pick` into the root fails on
  its staged changes, so apply the diff with `git apply` and commit by pathspec. Before fixing a red,
  run `git log origin/main..HEAD`: the main loop may have committed the fix already, and then the
  next goal end's push is the fix. Remove a scratch worktree from the root, never from inside it.
  A stale perf red is fixed by `bun nv proofs --record-perf --group <G>` in a scratch worktree at
  `HEAD`, after `bun install` and a copy of the root's ignored `tests/db/ca.crt` into it. Each class
  is its own group: `Core\Process\Result` is not inside `Core\Process`. The main loop may hold
  uncommitted ledger lines in the root, so never copy the ledger over. Take `git diff -U0` of it in the
  worktree and apply that in the root with `git apply --unidiff-zero`, never `--cached`: another
  session's commit takes what is staged. Check with `bun nv proofs --gate --limit 0`, then commit
  with `git commit -F <msg> -- data/perf/members.ndjson` at once. A growth check that fails on a
  figure whose code did not change is noise: measure that one again with `--id`. Run one attack with
  `bun nv proofs --run --id <feature> --show --nvs E:/mwl/target/release/nvs.exe`, never with a
  direct `nvs run`: an attack can change the checkout.

### Backlog

- The function-entry safepoint returns with no error edge, so a stop at entry may keep that
  frame's arguments. `crates/nvs-ir/src/lower/mod.rs:emit_safepoint` Moving the poll after the
  parameters are bound changes recursion backtraces. Take it only if a CI red or a test shows it.
- Not checked: on a system whose coreutils are one multicall binary behind links, such as Ubuntu
  26.04, `exec = ["/usr/bin/echo"]` may canonicalize to that binary. Then the grant covers
  `/usr/bin/rm` as well, because the door starts the path as written and the binary picks its tool
  from that name. `crates/nvs-config/src/capability.rs:path_granted` Check the layout on a 26.04
  runner before you change anything. A fix compares the written path too, and it is a security
  change, so give it to the main loop as a goal.
