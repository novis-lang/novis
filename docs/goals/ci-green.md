---
looper: 1
title: "the push run of main's tip is green"
summary: "Reached when origin/main holds every commit of the chain that a CI run builds, and the push runs of CI and Website on that commit ended green. The session pushes, watches the run, fixes every red job and pushes again until that holds."
context: [tools/nv/cmd/ci.ts, docs/rules/testing/every-run-runs-every-job.md, docs/rules/testing/one-run-on-main.md]
stages:
  - id: 1
    title: "the goal is verified once, then pushed, watched and fixed until the run is green"
    status: open
    gate: true
    context: [.github/workflows/ci.yml, tools/nv/cmd/deep.ts, docs/agent/testing.md]
checks:
  - id: main-tip-is-green
    stage: 1
    name: "origin/main holds every built commit of main, and its push runs of CI and Website ended green"
    run: [bun, nv, ci, --green]
    timeout: 300
---

# the push run of main's tip is green

## Why

The chain's last goal takes the full gate, and its `after_gate` hook pushes `main` with `bun nv ci
--push` (`looper.toml:114`). That push starts a `CI` run and a `Website` run, and nobody reads their
verdict: the next start of the loop begins either way. The full gate is `looper verify` on this
machine. `CI` runs more: Linux, macOS, aarch64 and Windows, under load, ASAN, Miri, the cost guards,
the build legs and the `dev` image (`rule:testing/every-run-runs-every-job`).

So a chain can end with every goal reached and `main` red on GitHub. This goal is last on the chain
and closes that. Its session pushes what is not pushed, watches the run, fixes each red job and
pushes again. It is reached only when the run on `main`'s tip is green.

The stop condition is `bun nv ci --green` (`tools/nv/cmd/ci.ts:1130`), which the reached goal
`chain-ends-green` built for this cycle. It exits 0 only when no push was refused, `origin/main` holds
the newest commit on local `main` that changes a file outside `NOTES_ONLY` (`tools/nv/cmd/ci.ts:257`),
and each workflow of `ONE_RUN_ON_MAIN` (`tools/nv/cmd/ci.ts:178`) that the commit's paths trigger has
a complete green push run on it. A run still going and a cancelled run are not green. It exits 1 when
`main` is not green and 2 when `gh` or `git` fails.

This goal moves no feature, so it owes the website nothing.

## Standing decisions

**This goal stays last on the chain.** A goal added later is placed before it with `looper goal move`.
It starts only when it is the live goal, so every other goal's commits are on `main` by then, and it
is the goal that takes the chain's full gate.

**CI decides, and no whole local lane runs first.** The session pushes and reads CI's verdict. It
never runs a whole `bun nv deep`: the full gate ran `looper verify` here, and a day of local runs
before a first push found no CI red. A fix is still checked before it is pushed, by the one local step
of the job it fixes (`bun nv deep --only <job>`, where the job has a step) and by `bun nv affected
--run`. Before each commit that touches Rust, `cargo fmt --all --check` and `cargo clippy
--all-targets -- -D warnings` are clean.

**One push carries every fix of a run.** When a job fails, the session lets the run finish, passing
`--past <job>` for each job it has already seen. It fixes every red job, then pushes once. A push
costs a full run of about 50 minutes, so a push per fix is the slow way.

**The push is `bun nv ci --check`, then `bun nv ci --push`.** Never `git push`, never a force push,
never another branch. A red pre-push check is fixed before the push: its checks are CI's own and take
two minutes. A push carries every commit on local `main`.

**There is no cap on pushes.** The cycle repeats until the run is green. The handoff's State counts
the pushes, each with its run's id and its red jobs, so a later session sees the history.

**A job that fails the same way after its fix was pushed is not guessed at again.** The first fix was
wrong, so the next one starts from evidence. The session reproduces the failure first: with `bun nv
deep --only <job>`, or with `bun nv linux-test` for a Linux job. A job that runs only on CI (macOS,
aarch64) cannot be reproduced here. For it, a `reader` subagent reads the whole log under
`.agent-tmp/ci/`, and the next push adds the output that tells the candidate causes apart, together
with the fix.

**A fix repairs the cause.** No test is deleted, renamed or loosened. No job is skipped, made
optional or given `continue-on-error`. No bound is raised until the job passes, except for a program
that waits on purpose. A check that is itself wrong is fixed as a check, and the commit says why
(`AGENTS.md` § *In the loop*).

**A failure no commit can fix is run again.** A runner that went away, a network error during a
download and a GitHub outage are rerun with `bun nv ci --rerun`. A test that fails the same way twice
is not noise.

**A red that a side loop's unfinished work causes is told to that loop.** The session sends it with
`bun tools/looper/main.ts loop say --loop <loop> "<job>: <what failed and why>"` and passes over that
job with `--past`. Every other red on `main` is this goal's to fix, whichever goal wrote the code.

**`BLOCKED` is for what needs a person.** A missing secret, a refused permission, a repository
setting, money, or a repair of this machine. The status line names each red job, its cause and what
was tried. Nothing else stops the cycle.

**No release and no fuzz search.** This goal never dispatches, approves, publishes or reruns a
`Release` or `Promote release images` run. It never runs `bun nv fuzz`
(`rule:testing/a-fuzz-search-gates-nothing`).

**The session works in the root, on `main`.** A side loop may edit the same tree. Commit with a
pathspec of your own files, run no tree-wide git command, and leave every file you did not write as
it is.

**The gate stage verifies once, and then runs the cycle.** Stage 1 starts with the full gate's
`looper verify` over every goal since the last full gate. After that its proof is CI itself, which
runs every job. A fix made during the cycle is checked as the decisions above say, and no further
`looper verify` runs.

**The run of the retirement commit is not this goal's.** Once the goal is reached, its file is
deleted and the roadmap loses its entry. The `after_gate` hook pushes that commit, and a run starts
on it. It builds the same code as the green run, and `bun nv ci --status` shows its verdict.

**Lanes:** one lane. The stage touches whatever the red jobs name.

## Stages

### 1. the goal is verified once, then pushed, watched and fixed until the run is green

**Does:** verifies once on this machine, then runs the cycle below until `bun nv ci --green` exits 0.

**Split:** red jobs whose causes are independent go to writing subagents on the session's model, one
after another. Each is told its failure, the files to touch and the local step to run, and commits
with a pathspec. None of them pushes. A log longer than the tail `--watch` prints is read by a
read-only `reader` subagent, from `.agent-tmp/ci/`. The session keeps the push, the watch and the
handoff.

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

Once verify is green, every session runs this cycle until the run is green or the context reaches
120k:

1. **Ask:** `bun nv ci --green`. On exit 0, write the handoff and the status line `DONE CI green on
   <sha>`.
2. **Push what is not pushed:** `bun nv ci --check`, each red fixed and committed, then `bun nv ci
   --push`. Count the push in the handoff's State.
3. **Watch:** `bun nv ci --watch --next` with `run_in_background` and a timeout of 7200000, then end
   your turn. Its exit code starts your next turn.
   - **Green:** go to step 1.
   - **A job failed:** let the run finish with `--past` for each job already seen. Then fix every red
     job as § *Standing decisions* says, one commit per fix. Check each fix with its local step and
     `bun nv affected --run`. Then go to step 2.
   - **Exit 4 or 5:** the watch found unpushed commits that a run would build. Go to step 2.
4. Once the context reaches 120k, write the handoff with what is fixed, what is pushed and which run
   is going. Write the status line `CONTINUE <the fixes>, pushed in <sha>`, and stop.

- [ ] `bun nv ci --green` exits 0 (`tools/nv/cmd/ci.ts:1130`).

## Handoff

### State

Not started. The goal runs once every goal before it on the chain is reached.

### Next

**Stage 1, from its first item.** `looper verify` with `run_in_background`, then the cycle from step 1.

### Backlog

- `tools/nv/cmd/ci.ts:329`: the pre-push attribution check runs `gen-attribution --check` only, and
  CI's job also runs `--check-c-deps`. Adding it to `PRE_PUSH` would catch a stale C-dependency
  ledger entry before the push.
