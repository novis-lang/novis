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
with the next goal end's push. Never `git push`, never `bun nv ci --push`, never a force push, and
never a push of another branch.

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

1. **Never push.** The next goal end pushes, and its push starts the runs the watch waits for.
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
4. **Hand off:** write the handoff with the fixes committed and not yet pushed, and the status line
   `CONTINUE <the fixes>, out with the next goal end`, and stop. The next session's watch waits for
   that push.

## Handoff

### State

`origin/main` is at `5dc6c26e3`, the push of `request-overhead`'s end. It carried the nine fixes
of the two sessions before, and its run cleared every job they fixed. Its Website run is green.
Its CI run, https://github.com/novis-lang/novis/actions/runs/39926481336, ended red in three jobs.
The nightly deep-lane run on `7b6ef53ee` was cancelled when this push came in. Before that it
showed only reds `5dc6c26e3` already fixed. Three fixes are committed and not pushed:

- `supply chain`: `cargo deny` found RUSTSEC-2026-0385 in `ctutils 0.4.3`, which `aes-gcm`
  pulls in. `a024f2253` moves it to `0.4.4` in both lock files. `cargo deny check advisories`
  passes locally. `deny.toml`'s RUSTSEC-2026-0206 entry warns "not encountered", because
  `rustybuzz` is only in `extensions/pdf/Cargo.lock`. That is a warning, and it is left as it is.
- `proofs` on all three platforms: six `about.md` pages ran 201 to 237 words, past the gate's
  200. They are the pages of `Core\Response::redirect`, `Core\Db::quoteIdentifier`, and the
  `queryAs` and `streamAs` of `Db\Connection` and `Db\Transaction`. `296755665` shortens them.
  `bun nv proofs --owed --limit 0` owes nothing. The gate still stops before the attacks run, so
  the LDAP `whoami: Timeout` attack has not had its second try.
- `test / windows-x86_64 (tests)`: `env_context`'s socket, job and schedule cases stopped with
  E0607, because `BUILTIN\Users` could write their trees under `target/tmp`. The runner's `D:`
  grants that group create rights in every folder, and `bun nv checkout-acl` removed only
  `Authenticated Users` and `Everyone`. `8ea16803f` removes `BUILTIN\Users` too and grants it
  `(OI)(CI)(RX)` back. Every served test binary after `env_context` writes under
  `CARGO_TARGET_TMPDIR` too, and this fixes them all at once. `ba8930b80`'s per-test
  `scratch_private` was the symptom fix. The new ACL was tested on a local folder built with the
  runner's grant, and has not run on a runner yet.

`.agent-tmp/wt-ci`, `.agent-tmp/wt-cig` and `.agent-tmp/wt-r` are empty scratch folders. Git no
longer lists them as worktrees. Each contains only a `.looper` directory that Windows will not
delete while looper runs. Delete all three when looper is stopped.

### Next

**Stage 1, the cycle.** Never push: `main` goes out at the next goal end.

- [ ] Watch for the next push. `tools/nv/cmd/ci.ts:265`
  Run `bun nv ci --watch --next`, so the red run on `5dc6c26e3` does not end the watch at once.
  If `test / windows-x86_64 (tests)` fails with E0607 again, read the `checkout-acl` step's log
  first. `tools/nv/cmd/checkout-acl.ts:91` If the Windows test binaries after `env_context` now
  run, expect new reds there: the earlier failures stopped those binaries before they ran.
- [ ] Once proofs runs its attacks, read the `proofs / linux-x86_64` log. `tests/hostile/core/Ldap-Connection/authenticate/01-guess-logins-and-passwords.nvs:1`
  A second LDAP `whoami: Timeout` is not noise. Compare the socket deadline with the slice yield
  commits `13004e4f9` and `2b292f862`.
- [ ] Fix and commit each other red job. `.github/workflows/ci.yml:1`
  A render red needs a binary built at `HEAD`: the root's `target/debug/nvs.exe` carries the main
  loop's uncommitted edits. Build one with `cargo build --bin nvs` in the scratch worktree.
  It takes about 15 minutes cold. Then render with `NVS_BIN` pointing at it.
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
  worktree, apply that with `git apply --cached --unidiff-zero` and again without `--cached`, then
  check with `bun nv proofs --owed --limit 0` and commit the index. Run one attack with
  `bun nv proofs --run --id <feature> --show --nvs E:/mwl/target/release/nvs.exe`, never with a
  direct `nvs run`: an attack can change the checkout.

### Backlog

- The function-entry safepoint returns with no error edge, so a stop at entry may keep that
  frame's arguments. `crates/nvs-ir/src/lower/mod.rs:emit_safepoint` Moving the poll after the
  parameters are bound changes recursion backtraces. Take it only if a CI red or a test shows it.
