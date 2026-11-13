---
looper: 1
title: "the push run of main's tip is green"
summary: "Reached when origin/main holds every commit of the chain that a CI run builds, and the push runs of CI and Website on that commit ended green. The session pushes, watches the run, fixes every red job and pushes again until that holds."
context: [tools/nv/cmd/ci.ts, docs/rules/testing/every-run-runs-every-job.md, docs/rules/testing/one-run-on-main.md]
stages:
  - id: 1
    title: one command says whether main's tip is green
    status: done
    context: [tools/nv/test/ci.test.ts, tools/nv/cmd/release.ts, docs/agent/commands.md]
  - id: 2
    title: "the goal is verified once, then pushed, watched and fixed until the run is green"
    status: open
    gate: true
    context: [.github/workflows/ci.yml, tools/nv/cmd/deep.ts, docs/agent/testing.md]
checks:
  - id: green-mode-exists
    stage: 1
    name: "bun nv ci names the --green mode in its usage"
    run: [bun, nv, ci]
    exit: nonzero
    output_contains: ["--green"]
    timeout: 120
  - id: main-tip-is-green
    stage: 2
    name: "origin/main holds every built commit of main, and its push runs of CI and Website ended green"
    run: [bun, nv, ci, --green]
    timeout: 300
---

# the push run of main's tip is green

## Why

Every goal on the chain ends with a push. looper's `after_retire` hook runs `bun nv ci --push`
(`looper.toml:107`), and that push starts a `CI` run and a `Website` run. Nobody reads their verdict.
The hook only warns when the pre-push checks refuse the push, and the next goal starts either way.
The loop's own gate is `looper verify` on this machine. `CI` runs more: Linux, macOS, aarch64 and
Windows, under load, ASAN, Miri, the cost guards, the build legs and the `dev` image
(`rule:testing/every-run-runs-every-job`).

So a chain can end with every goal reached and `main` red on GitHub. This goal is last on the chain
and closes that. Its session pushes what is not pushed, watches the run, fixes each red job and
pushes again. It is reached only when the run on `main`'s tip is green.

The stop condition has to be a command, and none exists. `bun nv ci --status` exits 0 while a run is
still going, and it does not ask whether local `main` is pushed
(`tools/nv/cmd/ci.ts:856`). `bun nv release --tested <sha>` needs a commit written into the check and
always exits 0 (`tools/nv/cmd/release.ts:1098`). Stage 1 builds the command.

## Standing decisions

**This goal stays last on the chain.** A goal added later is placed before it with `looper goal move`.
It starts only when it is the live goal, so every other goal's commits are on `main` by then.

**`bun nv ci --green` is the stop condition.** It is a new mode of `bun nv ci`. It exits 0 only when
all of these hold, and it prints one line for each that does not:

- No push was refused: the file `REFUSED` (`tools/nv/cmd/ci.ts:302`) does not exist.
- `origin/main` holds the built commit. The built commit is the newest commit on local `main` that
  changes a file outside `NOTES_ONLY` (`tools/nv/cmd/ci.ts:228`). A commit of the loop's notes alone
  changes nothing a run builds, so the handoff's own commit does not turn the verdict red.
- Each required workflow has a push run that is complete and green, on the built commit or on a later
  commit of `origin/main` that differs from it in notes alone. A run still going is not green. A
  cancelled run is not green.
- The required workflows are those of `ONE_RUN_ON_MAIN` (`tools/nv/cmd/ci.ts:149`), `CI` and
  `Website`. Stage 1 reads each workflow's `on:` block. A workflow whose push trigger has a path
  filter that the built commit did not match is not required for that commit. Safe default if that
  cannot be read from the workflow file: `CI` is required, and `Website`'s newest run must not be red.

It exits 1 when `main` is not green and 2 when `gh` or `git` fails. It reuses the judgement
`lookUpFullRun` already makes for a `CI` run (`tools/nv/cmd/release.ts:719`), so the two commands
cannot disagree about what a full run is. **What it spends:** a `git fetch` and a few `gh` calls per
check run. Trade-off: one more mode in `ci.ts`, paid in simplicity, for a stop condition that cannot
be reached by a run that is still going or by commits that were never pushed.

**CI decides, and no whole local lane runs first.** The session pushes and reads CI's verdict. It
never runs a whole `bun nv deep`: each goal before this one ran its own gate, and a day of local runs
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

**`BLOCKED` is for what needs a person.** A missing secret, a refused permission, a repository
setting, money, or a repair of this machine. The status line names each red job, its cause and what
was tried. Nothing else stops the cycle.

**No release and no fuzz search.** This goal never dispatches, approves, publishes or reruns a
`Release` or `Promote release images` run. It never runs `bun nv fuzz`
(`rule:testing/a-fuzz-search-gates-nothing`).

**The session works in the root, on `main`.** A side loop may edit the same tree. Commit with a
pathspec of your own files, run no tree-wide git command, and leave every file you did not write as
it is.

**The gate stage verifies once, and then runs the cycle.** Stage 2 starts with `looper verify`, which
covers stage 1's tool change on this machine. After that its proof is CI itself, which runs every
job. A fix made during the cycle is checked as the decision above says, and no further `looper
verify` runs.

**The run of the retirement commit is not this goal's.** Once the goal is reached, its file is
deleted and the roadmap loses its entry. The hook pushes that commit, and a run starts on it. It
builds the same code as the green run, and `bun nv ci --status` shows its verdict.

**Lanes:** one lane. Stage 1 touches `tools/nv/`, stage 2 touches whatever the red jobs name, and
stage 2 needs stage 1's command.

## Stages

### 1. one command says whether main's tip is green

**Does:** adds `bun nv ci --green`, as § *Standing decisions* says, with its tests and its help.

- [x] `tools/nv/cmd/ci.ts:1448`: the mode `--green` joins `modes` and the dispatch at
  `tools/nv/cmd/ci.ts:1472`. Its function takes `CiSeams`, as `status` does
  (`tools/nv/cmd/ci.ts:856`).
- [x] `tools/nv/cmd/ci.ts:776`: the built commit is found with the `NOTES_ONLY` filter that the
  unpushed-commit list already uses. One function serves both.
- [x] `tools/nv/cmd/release.ts:719`: the `CI` run is judged by `lookUpFullRun`'s rule, exported or
  moved so `ci.ts` calls it. A run still going and a cancelled run are not green.
- [x] `tools/nv/cmd/ci.ts:149`: the required workflows come from `ONE_RUN_ON_MAIN` and each
  workflow's own `on:` block under `.github/workflows/`, never from a list in the new function.
- [x] `tools/nv/cmd/ci.ts:1`: the header and the usage text name `--green` and its three exit codes.
- [x] `tools/nv/test/ci.test.ts:106`: a `describe("bun nv ci --green")` block with faked `gh` and
  `git`, one test for each of these:
  - a green complete run on the built commit exits 0
  - a run still going exits 1
  - a run with a red job exits 1 and names the job
  - a cancelled run exits 1
  - a built commit that `origin/main` lacks exits 1 and names it
  - a notes-only commit ahead of `origin/main` still exits 0
  - a green run on an older built commit exits 1
  - a refused push exits 1
  - a `gh` that fails exits 2
- [x] `docs/agent/commands.md:632`: the command list names `bun nv ci --green`.
- [x] `bun test tools/nv/test/ci.test.ts` is green. This stage's change is checked by that run, since
  a cargo build does not reach it.

### 2. the goal is verified once, then pushed, watched and fixed until the run is green

**Does:** verifies once on this machine, then runs the cycle below until `bun nv ci --green` exits 0.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

**Split:** red jobs whose causes are independent go to writing subagents on the session's model, one
after another. Each is told its failure, the files to touch and the local step to run, and commits
with a pathspec. None of them pushes. A log longer than the tail `--watch` prints is read by a
read-only `reader` subagent, from `.agent-tmp/ci/`. The session keeps the push, the watch and the
handoff.

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

- [ ] `bun nv ci --green` exits 0 (`tools/nv/cmd/ci.ts:1448`).

## Handoff

### State

Stage 1 is done. `bun nv ci --green` exists (`green` in `tools/nv/cmd/ci.ts`, under `// ---- --green`),
with ten tests in `describe("bun nv ci --green")` (`tools/nv/test/ci.test.ts:106`). The whole of
`ci.test.ts` is green, 69 tests. `outsideNotes` is now the one `NOTES_ONLY` filter, which
`unpushedCodeFiles` and the built-commit search share. `greenFullRun` was already exported from
`release.ts`, so `release.ts` is unchanged. `looper check --name green-mode-exists` passes.

The acceptance sweep's `env-secret-proofs` names no check in the tree (`looper check` finds none), and
the nightly run's cancelled jobs were cancelled by later pushes. Neither needs a fix.

Run live on `main` at `80331be10`, `--green` exits 1. CI push run 43517495984 is still going, with
two red jobs already: `extension sandbox probes` and `third-party attribution`. Website's push run is
green. Pushes of this goal so far: 0.

### Next

**Stage 2, the gate stage.** Commit stage 1 first if it is not on `main`.

- Run `looper verify` with `run_in_background`, and end the turn. Fix every failure in one pass, then
  run it once more. No test is expected to go red.
- Then run the cycle of stage 2. Start with `bun nv ci --watch --past "extension sandbox probes"
  --past "third-party attribution"`, so run 43517495984 finishes. Then read both failed logs under
  `.agent-tmp/ci/`. `third-party attribution` has a local step, `bun nv gen-attribution --check`.

### Backlog

Empty.
