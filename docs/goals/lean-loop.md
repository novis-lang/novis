---
looper: 1
title: "A session pays only for its own work, and parallel work does not conflict"
summary: "No debt run sits between two sessions, verify is red only for a real fault, the dead tooling is gone, a diagnostic code is never raced, a generated file never conflicts in a merge, the central tables are split per item, and the checks below pass."
context: [AGENTS.md, looper.toml, tools/nv/cmd/sweep.ts, tools/nv/cmd/verify.ts]
stages:
  - id: 1
    title: no debt run sits between two sessions
    status: done
    context: [tools/nv/cmd/disk.ts, tools/nv/select/record.ts, docs/goals/README.md]
  - id: 2
    title: verify is red only for a real fault
    status: done
    context: [crates/nvs-stdlib/tests/ldap.rs, crates/nvs-cli/tests/live_edit.rs]
  - id: 4
    title: the dead tooling and the stale state are gone
    status: open
    context: [tools/nv/main.ts, .claude/settings.json, tools/nv/cmd/guard.ts, .github/workflows/ci.yml]
  - id: 5
    title: a diagnostic code is never raced
    status: open
    context: [crates/nvs-diagnostics/src/lib.rs]
  - id: 6
    title: the central tables are split per item
    status: open
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-cli/src/main.rs, data/proofs/policy.json]
  - id: 7
    title: a generated file never conflicts in a merge
    status: open
    context: [.gitattributes, tools/nv/cmd/render.ts]
checks:
  - id: lint
    stage: 4
    name: "goals, chain, README and playbook lint clean"
    run: [looper, lint]
  - id: doctor
    stage: 4
    name: "looper finds no drift in the project's config"
    run: [looper, doctor]
  - id: doc-links
    stage: 4
    name: every doc link and anchor resolves
    run: [bun, nv, verify, --doc]
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, lean-loop]
    reach: true
---

# A session pays only for its own work, and parallel work does not conflict

## Why

A workflow audit of the last 100 loop sessions found that about 59% of a 37-minute cycle is not the
session's own work. The after-session `bun nv sweep` adds about 10.5 minutes to every session while
its debt grows. Sessions wait 6.4 minutes on average for verify, and `ldap` and `live_edit` make it
red in about one session in eight. AGENTS.md is two documents with about ten contradictions, and
several `nv` commands, a second guard hook and the plan's status block cost every session and are
read by nothing. The work itself has changed: goals are now small, 20 to 40 commits in under a day,
so a session's fixed cost matters more than it did. The last three stages remove what makes parallel
lanes conflict: in 6 of 28 side-branch merges, files a tool writes or tables every goal edits
conflicted. The last stage waits for looper's goal `landing-fixes`. This goal comes first on the chain because every later session gets cheaper.

## Standing decisions

**The user decided the scope and four calls:**
- The owed atoms are paid **at the goal end and by the nightly CI run**, never between sessions. If
  paying them at the goal end turns out to take hours, the owed benches move to the release gate,
  and the session says so in the handoff.
- **Generated files stay committed** and are rendered again on a merge, never merged by hand.
- **Side goal `docs-cleanup` carries what was this goal's stage 3 and the plan's deletion**, with the
  decisions about AGENTS.md and the playbook. The whole plan goes, its milestone machinery too.
- **Looper gets two fixes now**: a landing pauses only the loops in the root checkout, and a
  catch-up renders the generated files it would conflict on. Both are looper's goal
  `landing-fixes`. A chain planned as a graph, with lanes opened for it automatically, waits until
  this goal's savings are measured.

**These calls were the agent's, from the audit:**
- The flaky tests are fixed at their cause, never retried. A test that is red under load and green
  alone has a wrong deadline or a shared resource, and the stage finds which.
- A diagnostic code range is reserved per area, so two sessions can add codes in two areas without a
  race.
- The perf ledger `members.ndjson` merges with git's built-in `union` driver, because it is an
  append-only ledger. Goal `docs-cleanup` moves it from `docs/perf/` to `data/perf/`.
- `.claude/settings.json` loses only the second guard hook and entries that a wider entry already
  allows. If the permission classifier refuses that edit in the loop, the session names it in the
  handoff for the user to make, and goes on with the next item.

**No product behaviour changes.** A Novis program, its output and its messages stay the same in every
stage. Stage 6 moves code between files and changes no signature.

**Trade-offs.**
- Stage 1 saves about 10 minutes per session. A red in an owed atom can stay unseen for up to one
  goal, usually under a day, and the nightly CI run still finds it.
- Stage 2 costs nothing at run time. The two test binaries may take longer, if a fix waits on a
  real event in place of a timer.
- Stage 4 removes code. Every session reads fewer tokens.
- Stages 5 to 7 make central files smaller and add no memory. A per-item file is more files to list,
  and a generated file is rendered once more after every merge.

## Stages

### 1. no debt run sits between two sessions

**Does:** the after-session hook pays no debt, the owed atoms run once at the goal end, and the disk
is swept only when it is low.

- [x] The debt run and its budget leave `bun nv sweep`, which keeps the nightly read and a disk sweep. `tools/nv/cmd/sweep.ts:68`
- [x] The disk sweep deletes `target/deps` and `target/incremental` only below `nv disk`'s free-space line. `tools/nv/cmd/disk.ts:796`
- [x] The owed atoms run as one check that every goal shares, judged last, such as a floor check with `reach: true`. `docs/goals/README.md:1`
- [x] One run of that check is measured, and its time by atom kind is written into the debt command's module doc. `tools/nv/cmd/debt.ts:1`
  - [x] `looper check --name floor-debt` runs once, in the background, with nothing else building.
  - [x] The per-turn lines of `.loop/checks/floor-debt.log` give the time by atom kind.
  - [x] The figures go into `tools/nv/cmd/debt.ts`'s module doc; past two hours, the owed benches move to the release gate.
- [x] `looper.toml`'s hook comment and the `[jobs]` lists say what runs now. `looper.toml:55`
- [x] A Linux leg the debt cannot run, because its goal names no fixture, does not keep `floor-debt` red. `tools/nv/cmd/debt.ts:244`, `tools/nv/accept/legs.ts:331`
  - [x] `legsWait` gives why a goal's legs have nothing to run, and `payDebt` leaves those legs in the store, neither paid nor left.
  - [x] `tools/nv/test/debt.test.ts` pins both, and the module doc says why the leg is not recorded green.

### 2. verify is red only for a real fault

**Does:** `ldap` and `live_edit` pass under the load of a full `bun nv verify`, as they do alone.

- [x] The `ldap` socket deadlines are found and fixed at their cause. `crates/nvs-stdlib/tests/ldap.rs:277`, `crates/nvs-stdlib/tests/ldap.rs:1485`
  - [x] Two processes of the binary at once, as two worktrees' verify runs are, no longer clash on fixed entry names in the one Samba (`own`).
  - [x] The `Timeout` that is left, seen only with two processes and every core busy, is found at its cause.
  - [x] The 2-second floor of every Samba case is explained, and removed if it is a fallback in the connect.
  - [x] The `.nvst` cases that write to Samba name their entries with `Core\Os::pid()`, so two trees at once do not clash.
- [x] The `live_edit` 30-second swap is found and fixed at its cause. `crates/nvs-cli/tests/live_edit.rs:1010`
  - [x] The case is run under load until it fails, or until enough runs pass to rule load out.
  - [x] The cause is fixed without raising `BOUND`.
- [x] `verify.ts`'s module doc no longer lists either as a load failure. `tools/nv/cmd/verify.ts:1`

### 4. the dead tooling and the stale state are gone

**Does:** no command, hook, file or doc block remains that nothing reads, and local verify runs the
doc checks CI runs.

- [x] `nv pack`, `nv allow`, `nv audit`, `nv query` and `nv class-cards` are deleted with their tests and allowlist entries. `tools/nv/main.ts:23`
- [x] `bun nv guard`'s `proof-loop` rule moves into `looper.toml`'s `[[guard.deny]]`, and its hook and command are deleted. `.claude/settings.json:10`, `tools/nv/cmd/guard.ts:296`
- [ ] `nv links` and `nv layout` give way to `looper links` and `looper layout` in CI. Of `nv proofs --comments` and `looper prose`, the one that judges both Markdown and `.nvs` comments stays. `.github/workflows/ci.yml:974`
- [ ] CI's `docs` job checks run in `bun nv verify`, each only when what it reads changed. `.github/workflows/ci.yml:945`, `tools/nv/cmd/verify.ts:1`
- [ ] The stale references go: the `docs/decisions/README.md` sync skip, the owner-gate wording, the dead hooks in `commands.md`, and the settings entries a wider entry already allows. `looper.toml:79`, `tools/nv/cmd/owners.ts:14`, `docs/agent/commands.md:630`, `.claude/settings.json:130`
- [ ] The old driver's files under `.loop/` that nothing reads, and `tools/__pycache__/`, are deleted from the main checkout. `looper.toml:1`

### 5. a diagnostic code is never raced

**Does:** each area's codes live in their own module with a reserved range, and a test refuses a code
outside its module's range.

- [ ] The code table is split into one module per area, each with its range. `crates/nvs-diagnostics/src/lib.rs:140`
- [ ] A test refuses a code outside its module's range and a code used twice. `crates/nvs-diagnostics/src/lib.rs:1`
- [ ] `conventions.md`'s diagnostic shape says how to take the next code in an area. `docs/agent/conventions.md:1`

### 6. the central tables are split per item

**Does:** a goal that adds a `Core` class, a CLI command or a feature's proof policy adds a file and
edits no shared table by hand.

- [ ] The `Core` class table is one file per class, collected in one place. `crates/nvs-stdlib/src/registry.rs:1774`
- [ ] Each `nvs` command registers itself, so a new command edits no central `match`. `crates/nvs-cli/src/main.rs:144`
- [ ] `data/proofs/policy.json` is one file per feature. `data/proofs/policy.json:1`

### 7. a generated file never conflicts in a merge

**Does:** every file a tool writes is declared generated, and a merge renders it again.

- [ ] `.gitattributes` marks every rendered file generated, and the perf ledger `members.ndjson` as `merge=union`. `.gitattributes:43`
- [ ] `commands.md` says how a person resolves a merge conflict in a generated file: take either side, then render. `docs/agent/commands.md:1`
- [ ] Once looper's goal `landing-fixes` is reached, `looper.toml`'s `[merge]` table names every rendered file and the commands that render them. If it is not reached when this item comes up, the session writes `BLOCKED looper's goal landing-fixes is not reached`, because only the user starts looper's loop. `looper.toml:1`

## Handoff

### State

Stage 4's first two items are done.

- `nv pack`, `nv allow`, `nv audit`, `nv query` and `nv class-cards` are gone, with their tests,
  allowlist entries and doc mentions. `bun nv audit python` was the guard of
  `rule:tooling/the-repository-tools-are-one-bun-program`, and nothing ran it, so the rule no longer
  names it.
- `bun nv guard` and its hook are gone. Its `proof-loop`, `cargo-p` and `heavy-run` rules are four
  `[[guard.deny]]` regexes in `looper.toml`, pinned by `tools/nv/test/guard-deny.test.ts`, and
  `looper guard` was probed with them by hand. `whole-read` and `inline-write` are looper's
  built-ins, except two parts nothing covers now: a `cat` or `sed -n` of a big file through Bash, and
  a write from `python -c` or `bun -e`.

The nightly run 39013536848 is still the newest, and nothing in it is new.

### Next

Stage 4, CI's `docs` job and the doc checks in verify, on `.github/workflows/ci.yml` and `tools/nv/cmd/verify.ts`:
- [ ] Swap `nv links` and `nv layout` for `looper links` and `looper layout` in CI, and keep the one of `nv proofs --comments` and `looper prose` that judges both Markdown and `.nvs` comments. `.github/workflows/ci.yml:974`, `.github/workflows/ci.yml:976`
- [ ] Run CI's `docs` checks in `bun nv verify`, each only when what it reads changed. `.github/workflows/ci.yml:945`, `tools/nv/cmd/verify.ts:1`
- [ ] Remove the stale references: the `docs/decisions/README.md` sync skip, the owner-gate wording, the dead hooks in `commands.md`, and the settings entries a wider entry already allows. `looper.toml:103`, `tools/nv/cmd/owners.ts:14`, `docs/agent/commands.md:623`, `.claude/settings.json:120`

### Backlog

- `test_request_dispatches_in_process_through_the_compiled_route_table` was red once in the `covws` verify, also alone, and green on every rerun. The unclaimed path gave status 200 and an empty body, so `Core\Env::context()` was not `Request` in that run. `crates/nvs-cli/src/runner.rs:3410`, `crates/nvs-cli/tests/fixtures/runner/in-process-request.nvs:31`
- The Windows warm-start budget: 13 ms of work on a quiet hosted runner against 6 ms. A regression or the runner's file scanning; `tools/nv/cmd/bench.ts` § *The warm-start figure* owns it.
- `fullRun` and `runBenches` still take a `deadline` that no caller passes, and `runBenches` an `order` and `onStart` that only the debt passed. `tools/nv/select/full.ts:63`, `tools/nv/proofs/select.ts:265`
- The debt's progress items for the legs show `running` from the start of the run to its end, though their turn took 2 seconds. `tools/nv/cmd/debt.ts:250`
- `looper guard` matched a commit in looper's own checkout against a claim on mwl's `docs/goals/README.md`, because the claim check compares paths without their tree. Looper's `crates/looper-core/src/claims.rs` owns it.
- `Core\Ldap::connect` dials each address of a host in turn, so a `localhost` block pays 2 s on Windows when only `127.0.0.1` listens. `NvsTcp::connect_racing` would race them. `crates/nvs-stdlib/src/ldap/mod.rs:489`, `crates/nvs-host/src/net.rs:438`
