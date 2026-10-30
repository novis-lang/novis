---
looper: 1
title: "the Release workflow runs green on main, from plan to the image"
summary: "a dry-run Release run on main's tip ends green in every job, which a check reads from GitHub."
context: [docs/release.md, tools/nv/cmd/ci.ts]
stages:
  - id: 1
    title: "bun nv ci dispatches a full CI run and a dry-run Release, watches it, and reads its verdict"
    status: open
    context: [tools/nv/test/ci.test.ts, tools/nv/cmd/release.ts, .github/workflows/release.yml, docs/rules/testing/one-run-on-main.md]
  - id: 2
    title: "the full ci.yml run on main's tip is green in every job"
    status: open
    context: [.github/workflows/ci.yml, docs/rules/testing/ci-lanes.md, docs/rules/testing/the-deep-lane.md, docs/agent/testing.md]
  - id: 3
    title: "a dry-run Release run on main's tip is green in every job"
    status: open
    context: [.github/workflows/release.yml, .github/workflows/build.yml, .github/workflows/image.yml, docker/README.md]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: ci-release-tests
    stage: 1
    name: bun nv ci dispatches only a dry-run Release, watches a Release run without cancelling it, and judges a dry run as release.yml does
    run: [bun, test, tools/nv/test/ci.test.ts]
    tests:
      - dispatch dry-run starts release.yml on main with dry_run true, and no argument starts a real release
      - dispatch full starts ci.yml on main, and refuses while main has commits origin lacks
      - dispatch waits until GitHub lists the new run, and prints its address
      - watch release follows the newest Release run job by job, and cancels no run
      - release-status is green for a dry run on main whose every job passed and whose publish was skipped
      - release-status counts a skipped verify job only beside a green full ci.yml run of that commit
      - release-status is red when a commit after the dry run changes a file outside docs/goals and the playbook
      - release-status reads only dry runs, and the newest completed one decides
      - release-status names a failed or cancelled job and its step, and passes over a failed optional build leg
  - id: release-dry-run-green
    stage: 3
    name: the newest completed dry-run Release run on main is main's tip, and every job in it passed
    run: [bun, nv, ci, --release-status]
    reach: true
    floor: false
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, release-run]
    reach: true
    floor: false
---

# the Release workflow runs green on main, from plan to the image

## Why

`.github/workflows/release.yml` has run five times, from 2026-10-09 to 2026-10-24, and never ended
green. The reds were in its `verify` job (the proofs gate on three OSes, third-party attribution, the
Miri units, the chapter examples and the website check), and two runs were cancelled. So the code that
only a release runs has never executed: `build` at a version (`bun nv release --apply` in its
throwaway copy), the provenance attestation, `publish`, and the versioned image push. The full
`ci.yml` run that `test` stands for has been red or cancelled in each of its last eight dispatches.

The user's call (2026-10-30): the last goal on the chain is the release itself. It watches CI for as
long as the release build fails, and it ends when the release CI is green. After it, cutting the
first release is the user's clicks alone, on a commit that has already passed everything a dry run
runs.

A goal needs an exit code, and "the release is green" has one only if a command reads it from
GitHub. Stage 1 builds that command. Stage 2 gets the full suite green, because a dry run on a commit
with a green full run skips its `test` job (`bun nv release --tested`), so each later dry run takes
the build matrix and the images alone. Stage 3 gets the dry run green.

## Standing decisions

**The loop dispatches dry runs only.** It may start `release.yml` with `dry_run: true`, which writes
nothing, tags nothing and pushes no image. The real run, the `release` environment's approval and the
draft's publish click stay the user's. The goal ends before any of them, so no session of this goal
ever needs `BLOCKED` for them. `docs/release.md:8` says no agent may start a release, and stage 1
amends it to allow an agent's dry run and nothing else.

**The command is `bun nv ci`'s, not `bun nv release`'s** (AGENTS.md rule 12). `bun nv ci` already
pushes `main`, watches runs and reads jobs through `gh` and `git` seams a test replaces. `bun nv
release` is the decisions `release.yml` makes, it runs inside that workflow, and nothing in it starts
a workflow. The new modes:

- `bun nv ci --dispatch full` runs `gh workflow run ci.yml --ref main`. A `workflow_dispatch` runs
  every job (`FULL` in `ci.yml`).
- `bun nv ci --dispatch dry-run` runs `gh workflow run release.yml --ref main -f bump=patch -f
  dry_run=true`. Its argv is one constant. No flag, argument or variable can turn it into a real run,
  and a test pins that every argv it builds carries `dry_run=true`.
- Both refuse while local `main` has commits `origin/main` lacks, since the run would test
  `origin`'s tip and not the tree. Both then wait, polling as `--watch` does, until GitHub lists a
  run newer than the newest one before the dispatch, and print its address. So the `--watch` started
  after it reads the new run, never the last red one. Five minutes with no new run exits 2.
- `bun nv ci --watch --release` watches the newest `Release` run on `main` in place of `CI` and
  `Website`, with the same polling, failure report, `--max-min` and exit codes. It cancels no
  `Release` run, ever: a cancelled `publish` is the torn state `release.yml`'s `concurrency` comment
  exists to prevent. `--watch` without `--release` is unchanged.
- `bun nv ci --release-status` is the read-only verdict, and the goal's acceptance check. It never
  fetches, pushes, dispatches or cancels.

**Which run is a dry run.** `release.yml` gains `run-name: ${{ inputs.dry_run && 'Release (dry run)'
|| 'Release' }}`, so the runs API's `display_title` says it, and the Actions list shows it to the user
too. Safe fallback, if `run-name` cannot read the input: a run whose `publish` job (`tag, changelog
and draft release`) was skipped while every `build` leg passed counts as a dry run.

**What `--release-status` calls green.** It reads `actions/workflows/release.yml/runs?branch=main`,
not the 40 newest runs `--watch` lists. The newest *completed* dry run decides, and a newer one still
going is printed and does not. It is green when all of these hold, and it prints the run's address
and each job either way:

- Its `head_sha` is local `main`, or an ancestor of it where `git diff --name-only <sha> main` lists
  only paths under `docs/goals/` and `docs/agent/playbook/`. Equality with `origin/main` alone would
  pass with code fixes committed and never pushed. Equality with `main` alone would fail on the
  session's own handoff commit. A change to anything a release builds needs a new dry run.
- Every job concluded `success`, with three exceptions that mirror `release.yml`'s own `if:` lines.
  `publish` is `skipped`, as every dry run's is. The `verify` job (`test`) may be `skipped` only when
  `greenFullRun` (`tools/nv/cmd/release.ts:689`) finds a green full `ci.yml` run of that sha, the
  lookup `plan` makes with `--tested`. A failed leg that `build.yml` marks `optional: true`
  (`linux-x86_64-musl`, `windows-aarch64`) is named as a warning and does not hold the verdict, since
  `release.yml` ships without it and `publish` checks the required legs by name. Any other `skipped`,
  `cancelled` or unexpanded `${{ matrix.* }}` job is red.
- Exit 0 green, 1 red, 2 when `gh` or `git` cannot answer.

**Pushes go out every round.** This goal is about CI, so the rule that CI is pushed at goal ends only
does not apply to it. Every push is `bun nv ci --push`, whose pre-push checks must be green, and never
a bare `git push`. No push goes out while a full `ci.yml` run is going on `main`: `--push` cancels every
`CI` run on an older commit (`rule:testing/one-run-on-main`), and that includes a full run. A full
run another session's push cancelled is dispatched again. A `Release` run is never cancelled by
either command.

**One round per session.** A round is: read the reds, fix them, push, dispatch, start the watch with
`run_in_background`, write the handoff and the status line, and end the turn. The watch's report
starts the next turn. A watch that exits 3 at `--max-min` is started again. A session never sleeps
and never polls `gh` by hand. A round that does not fit one session hands off after its fixes are
committed, and the next session pushes.

**How a red is fixed.** Its cause, in one pass per round, never by loosening a check, renaming a test,
widening a bound or adding a skip (AGENTS.md § *In the loop*). A check that is itself wrong is in
scope, and its fix says why in the commit. A timing guard, such as `cost guards / windows-x86_64`'s
*Warm-start and core-scaling margins*, is fixed under `docs/agent/testing.md` § *A test holds on every
CI runner*. A flake is a bug with a cause, and it is fixed like one. `bun nv ci --rerun` is only for a
failure no fix can reach, such as a runner that went away, and the commit or handoff names it. A
Linux-only red is reproduced with `bun nv linux-test`, and a `docker` red with `tests/docker/smoke.sh`
(`docker/README.md`).

**Subagents.** Each red job's log goes to a `reader` subagent, several at once, one per job or per
group of shards. Each reports the cause as `file:line` and one line, never a log excerpt. Fixes whose
files are disjoint go to writing subagents side by side, none of them builds, and the session builds
once they all end. Every red on `main` is this goal's: it is last on the chain, so no other goal owns
one.

**`BLOCKED` is for GitHub alone.** An Actions outage, a billing or minutes limit, or a permission the
`gh` token lacks. Nothing a session can fix is a reason to stop.

**The gate stage and the dry run.** `--release-status` reads `main`, so a code fix the gate stage
commits turns it red. Such a fix goes through one more stage 3 round before `DONE`.

**The user's real run** is `docs/release.md` § *Cutting a release*, which stage 1 brings up to date.
The `DONE` status line names the green dry run's address and that section. The goal file is deleted
when the goal is reached, so nothing the user needs lives only here.

**The registry's verifier key is the user's.** `DEFAULT_KEY` (`crates/nvs-cli/src/package/manifest.rs:93`)
is `None`, so `nvs pkg` reads no archive from `registry.novis-lang.org`. The user makes the key pair
offline, keeps the private key, and commits the public key into that constant before the real run.
The loop never makes, reads or handles that key, and no session sets the constant. A key commit that
lands while this goal runs is code, so the next dry run covers it. One that lands after the goal is
covered by the real run's own `test` job, which runs the full suite for a commit with no green full run.

**Trade-offs.** Security: the command can start no real release, and it never cancels a `Release`
run. Latency and memory: nothing on the request path. Simplicity: four modes on an existing command,
with its seams and its tests, and one `run-name` line in the workflow. Cost: each round spends one
full matrix's runner minutes, which are free on a public repository.

**Lanes.** None. Every stage reads the same `main` and the same runs on GitHub, and only one session
drives CI at a time (`docs/agent/commands.md` § *The loop's hooks*). The stages run in order.

## Stages

### 1. bun nv ci dispatches a full CI run and a dry-run Release, watches it, and reads its verdict

**Does:** `bun nv ci --dispatch full|dry-run`, `--watch --release` and `--release-status` exist with
their tests, `release.yml` names a dry run, and `docs/release.md` allows an agent's dry run and
lists every click of the real one.

- [ ] `release.yml` gains its `run-name` after `name: Release`, and its header says an agent may dispatch a dry run with `bun nv ci --dispatch dry-run`, while only a person dispatches a real run. `.github/workflows/release.yml:66`, `.github/workflows/release.yml:3`
- [ ] `docs/release.md` says the same at its rule, and step 3's `dry_run` row names the command. § *Cutting a release* lists the user's whole first run: make the production registry's verifier key pair offline, keep the private key, and commit the public key into `DEFAULT_KEY` (today `None`, so `nvs pkg` reads nothing from `registry.novis-lang.org`); pause any loop; Actions → Release → `main`, `bump` `patch`, not a dry run; approve `release`; review and publish the draft; make the GHCR package public (§ 6: Packages → `novis` → Package settings → Change visibility → Public); then `docker pull ghcr.io/novis-lang/novis:<version>` and the website's install links. `docs/release.md:8`, `docs/release.md:149`, `crates/nvs-cli/src/package/manifest.rs:93`
- [ ] `Run` gains `display_title`, `listRuns` asks for it, and the watched workflows become a parameter, `["CI", "Website"]` by default and `["Release"]` under `--release`. `superseded` never names a `Release` run. `tools/nv/cmd/ci.ts:69`, `tools/nv/cmd/ci.ts:163`, `tools/nv/cmd/ci.ts:196`, `tools/nv/cmd/ci.ts:250`, `tools/nv/cmd/ci.ts:349`
- [ ] `--dispatch full|dry-run`: the two constant argvs, the refusal while `main` is ahead of `origin/main`, and the wait for the new run. `tools/nv/cmd/ci.ts:537`, `tools/nv/cmd/ci.ts:472`
- [ ] `--release-status`: one exported pure function judges a run, its jobs, the `ci.yml` runs of its sha and the paths changed since it, and a thin wrapper feeds it from `gh` and `git`. It imports `greenFullRun`. The optional legs are read from `build.yml`'s matrix, not copied. The real job names (`verify / …`, `build / …`, `container images (…)`, `tag, changelog and draft release`) are read once with `gh api` from run `40085277618` for the fixtures. `tools/nv/cmd/release.ts:689`, `.github/workflows/build.yml:89`, `tools/nv/cmd/ci.ts:507`
- [ ] The module doc, `summary` and `USAGE` describe the new modes, and `commands.md` names them beside `--watch`. `tools/nv/cmd/ci.ts:1`, `tools/nv/cmd/ci.ts:62`, `docs/agent/commands.md:631`
- [ ] The tests the `ci-release-tests` check names, in a new `describe` with the stubbed `gh` the file already uses. Run once by name: `bun test tools/nv/test/ci.test.ts`. `tools/nv/test/ci.test.ts:314`
- [ ] `bun nv ci --release-status` runs once against GitHub and exits 1, naming why: no dry run carries the new `run-name` yet. `tools/nv/cmd/ci.ts:537`

### 2. the full ci.yml run on main's tip is green in every job

**Does:** a `workflow_dispatch` run of `ci.yml` on `origin/main`'s tip ends green, so the dry runs
after it skip `test`. It takes as many rounds as it takes, one per session.

**Split:** each red job's log goes to a `reader` subagent, several at once. Fixes with disjoint files
go to writing subagents side by side, and the session builds once.

**The starting point** is whatever is red when the stage starts: `bun nv ci --status`, then `gh run
list --workflow ci.yml --event workflow_dispatch --limit 3` and `--event schedule`. Since
2026-10-30 the nightly runs on `main`. As of 2026-10-29 the reds were `cost guards / windows-x86_64`
(*Warm-start and core-scaling margins*), `proofs / linux-x86_64`, `proofs / windows-x86_64`,
`miri units (nvs-runtime 2/2)`, about eight `miri units (nvs-stdlib N/12)`, `overload past capacity`
(the flood smoke) and `ext templates` on three OSes. The newest dispatch then, `40919832286` on
`6d280a1ba`, failed in the first three. Six goals land before this one, so read the list again.

**Each round:** read the newest full run's reds; fix every cause in one pass; commit one cause per
commit; `bun nv ci --push`; `bun nv ci --dispatch full`; start `bun nv ci --watch` with
`run_in_background`; write the handoff with the reds still open; end the turn.

- [ ] The first round pushes stage 1's work and dispatches a full run, so every later round reads reds from a run on this goal's own commits. `tools/nv/cmd/ci.ts:472`
- [ ] The handoff lists every red job of the newest full run, grouped by cause, each with the `file:line` its `reader` named. `.github/workflows/ci.yml:1`
- [ ] Every cause is fixed and pushed, one commit per cause. `.github/workflows/ci.yml:1`
- [ ] `bun nv release --tested <origin/main's sha>` prints `tested=true`: a green full run of `main`'s tip exists. `tools/nv/cmd/release.ts:1030`

### 3. a dry-run Release run on main's tip is green in every job

**Does:** `bun nv ci --release-status` exits 0. The dry run exercises what only a release runs and a
dry run can reach: the version `--apply` in `build`, the attestations, the `.vsix` at the version, and
each image built and smoke-tested without a push.

**Split:** as stage 2's.

**Each round:** `bun nv ci --dispatch dry-run`; start `bun nv ci --watch --release` with
`run_in_background`; end the turn. On a red: fix every cause in one pass, `bun nv ci --push`, and
dispatch again. If the fix changed what the suites test, the new commit has no green full run, so the
dry run's `verify` job runs the full suite itself, and its reds are read the same way.

- [ ] The first dry run is dispatched on the commit stage 2 left green, with no push in between, so its `verify` job is skipped. `.github/workflows/release.yml:210`
- [ ] The fixtures of the `ci-release-tests` check are compared with the job names the first dry run shows, and corrected in the same slice if they differ. `tools/nv/test/ci.test.ts:314`
- [ ] Every red of `plan`, `build`, the `.vsix` and `docker` is fixed at its cause, one commit per cause. `.github/workflows/build.yml:1`, `.github/workflows/image.yml:1`, `tools/nv/cmd/release.ts:821`
- [ ] `bun nv ci --release-status` exits 0. `tools/nv/cmd/ci.ts:537`

### 4. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass. A code fix it commits needs one more stage 3
round, because `--release-status` reads `main`.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built. The goal file is written, and stage 1 is next. The newest dry-run detection,
`--release-status`, and the `run-name` do not exist yet. Release runs so far: five, the newest
`40098517153` (cancelled, 2026-10-24) and `40085277618` (failed in `verify`, 2026-10-24).

### Next

Stage 1, one group: `tools/nv/cmd/ci.ts`, `tools/nv/test/ci.test.ts`, `.github/workflows/release.yml`
and `docs/release.md`. No gate: the goals before this one on the chain are reached by the time it is
live.

- [ ] `release.yml`'s `run-name` and header. `.github/workflows/release.yml:66`, `.github/workflows/release.yml:3`
- [ ] `docs/release.md` allows an agent's dry run and lists the user's whole first run, the registry verifier key and the GHCR visibility switch included. `docs/release.md:8`, `docs/release.md:149`, `crates/nvs-cli/src/package/manifest.rs:93`
- [ ] The watched workflows as a parameter, `display_title`, and no `Release` run in `superseded`. `tools/nv/cmd/ci.ts:69`, `tools/nv/cmd/ci.ts:196`, `tools/nv/cmd/ci.ts:250`, `tools/nv/cmd/ci.ts:349`
- [ ] `--dispatch full|dry-run`. `tools/nv/cmd/ci.ts:537`
- [ ] `--release-status` with `greenFullRun`. `tools/nv/cmd/release.ts:689`, `tools/nv/cmd/ci.ts:507`
- [ ] Module doc, `USAGE`, `commands.md`. `tools/nv/cmd/ci.ts:1`, `docs/agent/commands.md:631`
- [ ] The nine tests, run once by name. `tools/nv/test/ci.test.ts:314`

No test is expected to go red.

### Backlog
