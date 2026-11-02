---
looper: 1
title: "the first release is published from main, from plan to the public image"
summary: "a real Release run on main ends green in every job, its draft is published and promoted, and a check reads all of it from GitHub."
context: [docs/release.md, tools/nv/cmd/ci.ts]
stages:
  - id: 1
    title: "bun nv ci dispatches a full CI run and a dry-run Release, watches it, and reads its verdict"
    status: done
    context: [tools/nv/test/ci.test.ts, tools/nv/cmd/release.ts, .github/workflows/release.yml, docs/rules/testing/one-run-on-main.md]
  - id: 2
    title: "the full ci.yml run on main's tip is green in every job"
    status: done
    context: [.github/workflows/ci.yml, docs/rules/testing/ci-lanes.md, docs/rules/testing/the-deep-lane.md, docs/agent/testing.md]
  - id: 3
    title: "a dry-run Release run on main's tip is green in every job"
    status: open
    context: [.github/workflows/release.yml, .github/workflows/build.yml, .github/workflows/image.yml, docker/README.md]
  - id: 4
    title: "bun nv ci cuts a real release: dispatch, approve, publish, and the published verdict"
    status: open
    context: [tools/nv/cmd/ci.ts, tools/nv/test/ci.test.ts, docs/release.md, .github/workflows/release.yml, .github/workflows/release-promote.yml]
  - id: 5
    title: "the goal is verified once, a dry run on the final tip is green, and the real release is published"
    status: open
    gate: true
    context: [docs/release.md, .github/workflows/release.yml, .github/workflows/release-promote.yml]
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
    name: the newest completed Release run on main, dry or real, is main's tip or the commit it tagged, and every job in it passed
    run: [bun, nv, ci, --release-status]
    reach: true
    floor: false
  - id: ci-real-release-tests
    stage: 4
    name: bun nv ci starts a real release only after a green dry run of main's tip, approves only green builds, and publishes only a whole draft
    run: [bun, test, tools/nv/test/ci.test.ts]
    tests:
      - dispatch release starts release.yml on main with bump patch and dry_run false, and only this mode does
      - dispatch release refuses unless release-status is green for main's tip, and while a Release run is going
      - approve-release approves the release environment of the newest real run only when every job before publish passed
      - approve-release refuses a dry run, a run with a failed required build leg, and a run not waiting on release
      - publish-release publishes the draft only when it carries every required leg's archive and the vsix, and docker passed
      - release-status is green after a real run when main's tip is the commit it tagged, or past it by notes only
      - release-published is green when the release is not a draft, its tag is on main, the promote run passed and the image pulls with no login
      - release-published names a private package as the one step left to the user
  - id: release-published
    stage: 5
    name: the newest release is published, its tag is on main, its images are promoted, and its version tag pulls with no login
    run: [bun, nv, ci, --release-published]
    reach: true
    floor: false
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, release-run]
    reach: true
    floor: false
---

# the first release is published from main, from plan to the public image

## Why

`.github/workflows/release.yml` has run five times, from 2026-10-09 to 2026-10-24, and never ended
green. The reds were in its `verify` job (the proofs gate on three OSes, third-party attribution, the
Miri units, the chapter examples and the website check), and two runs were cancelled. So the code that
only a release runs has never executed: `build` at a version (`bun nv release --apply` in its
throwaway copy), the provenance attestation, `publish`, and the versioned image push. The full
`ci.yml` run that `test` stands for has been red or cancelled in each of its last eight dispatches.

The user's call (2026-10-30): the last goal on the chain is the release itself. It watches CI for as
long as the release build fails. The user's call (2026-11-01): the goal ends with the full release,
not with the green dry run. So the loop cuts `0.0.1` itself, approves it, publishes the draft, and
reads the published release back from GitHub.

A goal needs an exit code, and "the release is out" has one only if a command reads it from GitHub.
Stage 1 builds the dry-run half of that command. Stage 2 gets the full suite green, because a dry run
on a commit with a green full run skips its `test` job (`bun nv release --tested`), so each later dry
run takes the build matrix and the images alone. Stage 3 gets the dry run green. Stage 4 builds the
real-run half: dispatch, approve, publish and the published verdict. Stage 5 verifies the goal once,
gets a dry run green on the final tip, and runs the real release.

## Standing decisions

**The loop cuts the real release, once, in the gate stage.** Stages 1 to 4 dispatch dry runs only,
which write nothing, tag nothing and push no image. The gate stage starts the one real run, after
verify is green and a dry run of `main`'s tip is green. The `gh` token on this machine is
`brainfoolong`, the `release` environment's one required reviewer (`prevent_self_review` is off), so
the loop approves the run and publishes the draft through `gh` with no click. Stage 4 amends
`docs/release.md:8`: an agent starts a real run only when the live goal says so, and only through
`bun nv ci --dispatch release`. The version is `0.0.1`, from `bump=patch` with no earlier tag, and
`bun nv release --preview patch` prints it before the dispatch. Any other version is a bug, fixed
before the run.

**The real run's order, in the gate stage.** Each step waits for the one before it to be green.

1. `looper loop pause` holds every other loop, so no session commits to `main` between the dispatch
   and `publish`'s push (`docs/release.md` § *Cutting a release*, step 3). The session commits and
   pushes its own handoff first, then commits nothing until `publish` has pushed.
2. `bun nv ci --dispatch release`, and `bun nv ci --watch --release` in the background.
3. When the run waits on `release`, the session reads the notes in the run summary, then runs `bun nv
   ci --approve-release`.
4. When the run ends green, `bun nv ci --publish-release`. That starts `release-promote.yml`, which
   moves `latest` and `MAJOR.MINOR` and dispatches `pages.yml`. The session watches both to green.
5. `git fetch` and `git merge --ff-only origin/main` bring `publish`'s version commit into local `main`,
   then `looper loop resume`.
6. `bun nv ci --release-published` exits 0, and the `DONE` line names the release's address.

A real run that fails before `publish` wrote nothing: fix the cause, push, get a new dry run green,
and dispatch again. A run that fails after `publish` pushed is never dispatched again, because its tag
exists. It is repaired by `docs/release.md` § *When it goes wrong*: a job rerun or a `release-promote`
dispatch with the tag. No command here ever cancels a `Release` run.

**The package's visibility is the one click.** GitHub has no API that makes a GHCR package public, and
a new package is private (`docs/release.md` § 6). If `--release-published` finds every other part green
and the version tag does not pull with no login, the session writes `BLOCKED make the GHCR package
novis public: Packages → novis → Package settings → Change visibility → Public`. That is the only
`BLOCKED` the release itself may need. The next session runs `--release-published` again and writes
`DONE`.

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
- Stage 4 adds the real-run modes, each with the same `gh` and `git` seams and exit codes:
  - `bun nv ci --dispatch release` runs `gh workflow run release.yml --ref main -f bump=patch -f
    dry_run=false`. It is the only argv with `dry_run=false`, and a test pins that. It refuses unless
    `--release-status`'s verdict is green for `main`'s tip, while any `Release` run is going, and
    while `main` has commits `origin/main` lacks outside `NOTES_ONLY`.
  - `bun nv ci --approve-release` approves the `release` environment of the newest real `Release`
    run, through `gh api repos/{repo}/actions/runs/{id}/pending_deployments` with `state=approved`.
    It refuses a dry run, a run not waiting on `release`, and a run where `plan`, `test` or a required
    `build` leg did not pass. It prints the run's notes before it approves.
  - `bun nv ci --publish-release` runs `gh release edit <tag> --draft=false` on the run's draft. It
    refuses unless that run's every job passed, `docker` included, and the draft carries every
    required leg's archive and `nvs-<version>.vsix`. The required legs are read from `build.yml`'s
    matrix, as `--release-status` reads them.
  - `bun nv ci --release-published` is the read-only verdict on the release, and the goal's last
    check. It is green when the newest GitHub release is not a draft, its tag's commit is on
    `origin/main`, the `Promote release images` run for that tag passed, and
    `ghcr.io/novis-lang/novis:<version>` has a manifest with no login. It asks for an anonymous token
    at `https://ghcr.io/token?scope=repository:novis-lang/novis:pull`, then reads
    `/v2/novis-lang/novis/manifests/<version>`. A `401` or `403` there, with everything else green,
    prints the visibility click as the one step left.

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
- From stage 4 on it reads real runs too, and the newest completed `Release` run decides, dry or
  real. A real run's `publish` and `docker` must pass, and its `head_sha` rule is measured from the
  commit its tag names. `publish` pushes that commit onto `main`, so after the release `main`'s tip is
  the tagged commit or past it by notes only. So the check stays green at the goal's end, after the
  real run.

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

**`BLOCKED` is for GitHub alone.** An Actions outage, a billing or minutes limit, a permission the
`gh` token lacks, or the package visibility click above. Nothing a session can fix is a reason to stop.

**The gate stage and the dry run.** `--release-status` reads `main`, so a code fix the gate stage
commits turns it red. Such a fix goes through one more dry run before the real run is dispatched. The
gate stage's verify runs before that dry run, so the real run builds exactly the tree verify passed.

**What the user reads afterwards** is `docs/release.md` § *Cutting a release*, which stage 4 brings up
to date: the steps a goal's agent takes, and the next release's run. The `DONE` status line names the
release's address. The goal file is deleted when the goal is reached, so nothing the user needs lives
only here.

**The registry's verifier key is pinned** (`613573f75`). The user asked on 2026-11-01 for the key to
be made then, and `nvs registry key --name registry.novis-lang.org` made it. The public key is
`DEFAULT_KEY` (`crates/nvs-cli/src/package/manifest.rs:93`), and a unit test pins that it parses under
the registry's host. The signer key is in the user's home folder, outside the repository. No session
reads, copies or moves it. The commit is code, so the next dry run's `verify` job runs the full suite
on it.

**Trade-offs.** Security: the real run still waits on the `release` environment, which the command
approves only after every build passed and the notes were read. The command never cancels a `Release`
run, and the registry's signer key never enters the tree. Latency and memory: nothing on the request
path. Simplicity: eight modes on an existing command, with its seams and its tests, and one `run-name`
line in the workflow. Cost: each round spends one full matrix's runner minutes, which are free on a
public repository.

**Lanes.** None. Every stage reads the same `main` and the same runs on GitHub, and only one session
drives CI at a time (`docs/agent/commands.md` § *The loop's hooks*). The stages run in order.

## Stages

### 1. bun nv ci dispatches a full CI run and a dry-run Release, watches it, and reads its verdict

**Does:** `bun nv ci --dispatch full|dry-run`, `--watch --release` and `--release-status` exist with
their tests, `release.yml` names a dry run, and `docs/release.md` allows an agent's dry run and
lists every click of the real one.

- [x] `release.yml` gains its `run-name` after `name: Release`, and its header says an agent may dispatch a dry run with `bun nv ci --dispatch dry-run`, while only a person dispatches a real run. `.github/workflows/release.yml:66`, `.github/workflows/release.yml:3`
- [x] `docs/release.md` says the same at its rule, and step 3's `dry_run` row names the command. § *Cutting a release* lists the user's whole first run: make the production registry's verifier key pair offline, keep the private key, and commit the public key into `DEFAULT_KEY` (today `None`, so `nvs pkg` reads nothing from `registry.novis-lang.org`); pause any loop; Actions → Release → `main`, `bump` `patch`, not a dry run; approve `release`; review and publish the draft; make the GHCR package public (§ 6: Packages → `novis` → Package settings → Change visibility → Public); then `docker pull ghcr.io/novis-lang/novis:<version>` and the website's install links. `docs/release.md:8`, `docs/release.md:149`, `crates/nvs-cli/src/package/manifest.rs:93`
- [x] `Run` gains `display_title`, `listRuns` asks for it, and the watched workflows become a parameter, `["CI", "Website"]` by default and `["Release"]` under `--release`. `superseded` never names a `Release` run. `tools/nv/cmd/ci.ts:69`, `tools/nv/cmd/ci.ts:163`, `tools/nv/cmd/ci.ts:196`, `tools/nv/cmd/ci.ts:250`, `tools/nv/cmd/ci.ts:349`
- [x] `--dispatch full|dry-run`: the two constant argvs, the refusal while `main` is ahead of `origin/main`, and the wait for the new run. `tools/nv/cmd/ci.ts:537`, `tools/nv/cmd/ci.ts:472`
- [x] `--release-status`: one exported pure function judges a run, its jobs, the `ci.yml` runs of its sha and the paths changed since it, and a thin wrapper feeds it from `gh` and `git`. It imports `greenFullRun`. The optional legs are read from `build.yml`'s matrix, not copied. The real job names (`verify / …`, `build / …`, `container images (…)`, `tag, changelog and draft release`) are read once with `gh api` from run `40085277618` for the fixtures. `tools/nv/cmd/release.ts:689`, `.github/workflows/build.yml:89`, `tools/nv/cmd/ci.ts:507`
- [x] The module doc, `summary` and `USAGE` describe the new modes, and `commands.md` names them beside `--watch`. `tools/nv/cmd/ci.ts:1`, `tools/nv/cmd/ci.ts:62`, `docs/agent/commands.md:631`
- [x] The tests the `ci-release-tests` check names, in a new `describe` with the stubbed `gh` the file already uses. Run once by name: `bun test tools/nv/test/ci.test.ts`. `tools/nv/test/ci.test.ts:314`
- [x] `bun nv ci --release-status` runs once against GitHub and exits 1, naming why: no dry run carries the new `run-name` yet. `tools/nv/cmd/ci.ts:537`

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

- [x] The first round pushes stage 1's work and dispatches a full run, so every later round reads reds from a run on this goal's own commits. `tools/nv/cmd/ci.ts:472`
- [x] The handoff lists every red job of the newest full run, grouped by cause, each with the `file:line` its `reader` named. `.github/workflows/ci.yml:1`
- [x] Every cause is fixed and pushed, one commit per cause. `.github/workflows/ci.yml:1`
- [x] `bun nv release --tested <origin/main's sha>` prints `tested=true`: a green full run of `main`'s tip exists. `tools/nv/cmd/release.ts:1030`

### 3. a dry-run Release run on main's tip is green in every job

**Does:** `bun nv ci --release-status` exits 0. The dry run exercises what only a release runs and a
dry run can reach: the version `--apply` in `build`, the attestations, the `.vsix` at the version, and
each image built and smoke-tested without a push.

**Split:** as stage 2's.

**Each round:** `bun nv ci --dispatch dry-run`; start `bun nv ci --watch --release` with
`run_in_background`; end the turn. On a red: fix every cause in one pass, `bun nv ci --push`, and
dispatch again. If the fix changed what the suites test, the new commit has no green full run, so the
dry run's `verify` job runs the full suite itself, and its reds are read the same way.

- [x] The first dry run is dispatched. It runs on `df536b806`, since another session's CI cache commit `9d94ca78a` had to be pushed first, so its `verify` job runs the full suite itself. `.github/workflows/release.yml:210`
- [x] The fixtures of the `ci-release-tests` check are compared with the job names the first dry run shows, and corrected in the same slice if they differ. `tools/nv/test/ci.test.ts:314`
- [ ] Every red of `plan`, `build`, the `.vsix` and `docker` is fixed at its cause, one commit per cause. `.github/workflows/build.yml:1`, `.github/workflows/image.yml:1`, `tools/nv/cmd/release.ts:821`
- [ ] `bun nv ci --release-status` exits 0. `tools/nv/cmd/ci.ts:537`

### 4. bun nv ci cuts a real release: dispatch, approve, publish, and the published verdict

**Does:** the four real-run modes the standing decisions name exist with their tests, and
`--release-status` reads a real run. `docs/release.md` and `release.yml`'s header say an agent may run
the real release under a live goal that says so. Nothing in this stage dispatches a real run.

**Split:** one writing session. `ci.ts` and its test file are one file set.

- [ ] `--dispatch release`: the third constant argv, and its refusals, which reuse `--release-status`'s pure judge and the `NOTES_ONLY` check. `tools/nv/cmd/ci.ts:472`, `tools/nv/cmd/ci.ts:621`
- [ ] `--release-status` reads real runs. The newest completed run decides, `publish` and `docker` must pass in a real run, and the path rule starts from the tagged commit. `tools/nv/cmd/ci.ts:537`
- [ ] `--approve-release` and `--publish-release`: a pure judge each, fed by a thin `gh` wrapper. The job names and the draft's asset names are read once with `gh api` from the first dry run for the fixtures. `tools/nv/cmd/ci.ts:507`, `.github/workflows/build.yml:89`
- [ ] `--release-published`: the release, its tag on `origin/main`, the promote run, and the anonymous GHCR manifest read through a `fetch` seam the test replaces. `tools/nv/cmd/ci.ts:537`
- [ ] `docs/release.md:8` and `.github/workflows/release.yml:3` allow an agent's real run under a live goal that says so. § *Cutting a release* names the command for each step and keeps the visibility click as the person's. The module doc, `summary`, `USAGE` and `docs/agent/commands.md:631` name the new modes. `docs/release.md:139`
- [ ] The tests the `ci-real-release-tests` check names, beside the stage 1 ones. Run once by name: `bun test tools/nv/test/ci.test.ts`. `tools/nv/test/ci.test.ts:314`

### 5. the goal is verified once, a dry run on the final tip is green, and the real release is published

**Does:** the stages before this one checked with a build alone, so this stage verifies the whole goal
once and fixes every failure in one pass. Then a dry run of the final tip goes green, and the real run
follows it in the order the standing decisions give.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds. The release steps after verify
are one session's, one step per turn, each waiting on its watch.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.
- [ ] `bun nv ci --check`, `bun nv ci --push`, `bun nv ci --dispatch dry-run`, and the dry run ends green: `bun nv ci --release-status` exits 0 on `main`'s tip. `tools/nv/cmd/ci.ts:537`
- [ ] `bun nv release --preview patch` prints `0.0.1`. `looper loop pause`, the handoff committed and pushed, then `bun nv ci --dispatch release` and the watch in the background. `tools/nv/cmd/release.ts:1030`
- [ ] The run waits on `release`: the notes are read, then `bun nv ci --approve-release`. `.github/workflows/release.yml:263`
- [ ] The run ends green: `bun nv ci --publish-release`, then the `Promote release images` and `pages.yml` runs end green. `.github/workflows/release-promote.yml:37`
- [ ] Local `main` fast-forwards to `origin/main`, and `looper loop resume`.
- [ ] `bun nv ci --release-published` exits 0, or names the visibility click and the session writes the one `BLOCKED` the standing decisions allow. `tools/nv/cmd/ci.ts:537`

## Handoff

### State

Stages 1 and 2 are done. Full run `41346116301` on `ec40af25e` is green in all 56 jobs, and
`bun nv release --tested` printed `tested=true` for it. The macOS CPU-stop cause is fixed in
`01ac9f143` (`wake_on_time`, `crates/nvs-host/src/watchdog.rs:1461`).

Stage 3: dry run `41352454403` on `df536b806` passed every job, its `verify` job running the full
suite. Only the optional `build / build windows-aarch64` failed, in its `Build` step. It does not
stand for `main`, since `613573f75` (the registry key) is code.

`eb9358fdb` fixes `--watch --release`: GitHub sets a run's `name` from `run-name`, so the watch
looked for `Release` and never saw `Release (dry run)`. Runs are now named from their `path`
(`workflowOf`, `tools/nv/cmd/ci.ts`). The test fixture's image jobs are `(debian)`, as on GitHub.

Dry run `41371737071` on `eb9358fdb` is going, with `verify` running the full suite again. Its
`plan`, `lint`, `docs`, `asan`, `tsan` and `container images` jobs had passed when this session
started its watch, `bun nv ci --watch --release --max-min 110`, in the background.

The acceptance sweep's reds were the stale ones the last handoff named: nightly `41295327777` on
`ef0d5cd3b`, and `env-secret-proofs`, which has no definition and no log.

`build / build windows-aarch64` cannot build: `corosensei` 0.3.4, the newest release, has no
context-switch backend for `aarch64` on Windows (its `src/arch/mod.rs:160` excludes it, and line 177 is
`compile_error!("Unsupported target")`). `Cargo.toml:122` and `crates/nvs-host/Cargo.toml:44` pull it.
The leg is `optional: true`, so the release ships without it. No change was made while the dry run is
going, since a `build.yml` edit would stop it standing for `main`.

### Next

- [ ] Read the watch's report. A watch that exits 3 is started again. On a red: one `reader` per red job, fix every cause in one pass, one commit each, `bun nv ci --push`, `bun nv ci --dispatch dry-run`, the watch in the background. `tools/nv/cmd/ci.ts:621`
- [ ] On green: `bun nv ci --release-status` exits 0, then stage 4. `tools/nv/cmd/ci.ts:537`

No test is expected to go red.

### Backlog

- `build / build windows-aarch64` fails at every run until `corosensei` gains a Windows `aarch64`
  backend, or the leg is dropped from `.github/workflows/build.yml:90`. An in-tree backend is unsafe
  stack-switching code with Windows ARM64 unwinding, which security outranks, so it waits for upstream.
  Decide after the release: drop the leg, or keep it as a known red warning.

- `cargo clippy` on macOS warns `chunks_exact` with a constant chunk size at
  `crates/nvs-runtime/src/child_watch.rs:845`. CI's `lint` job runs on Linux, so the macOS-only code
  is linted nowhere.

- `nvs info --licenses | head` panics with "failed printing to stdout" once the pipe closes. A CLI
  command that prints should end quietly on a closed stdout.
- `crates/nvs-server/src/serve.rs` is 14,400 lines, and `nvs-stdlib` is the largest crate: a split waits for a measured cost (`docs/agent/coding.md`).
- The website builds twice per push, in `.github/workflows/pages.yml` and in `ci.yml`'s `proofs` job (`rule:testing/ci-lanes`).
- `tools/nv/cmd/ci-changes.ts`'s `proofs` filter has no `extensions/`, so `bun nv pdf-validate` may not run on a change only to a component. That CI step and its no-Docker path have not run yet: read its first CI run, which stage 2's full run is.
