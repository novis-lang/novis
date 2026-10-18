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
    status: done
    context: [tools/nv/main.ts, .claude/settings.json, tools/nv/cmd/guard.ts, .github/workflows/ci.yml]
  - id: 5
    title: a diagnostic code is never raced
    status: done
    context: [crates/nvs-diagnostics/src/code/mod.rs]
  - id: 6
    title: the central tables are split per item
    status: done
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-cli/src/main.rs, data/proofs/policy.json]
  - id: 7
    title: a generated file never conflicts in a merge
    status: done
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

**The user decided the scope and these calls:**
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
- **The project runs and is developed the same way without looper.** Only a few developers have
  looper, and CI never does. So every check is written once, in `bun nv`, and looper runs the
  project's command where it has a check of its own. `nv links` and `nv layout` stay, and stage 4's
  swap to `looper links` and `looper layout` is dropped. Goal `works-without-looper` closes the
  gaps where only looper checks today.

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
- [x] `looper prose` judges the Markdown and `nv proofs --comments` the `.nvs` comments, because `looper prose` reads a whole `.nvs` file as prose. `looper.toml:89`
- [x] CI's `docs` job checks run in `bun nv verify`, each only when what it reads changed. `.github/workflows/ci.yml:945`, `tools/nv/cmd/verify.ts:151`
- [x] The stale references go: the owner-gate wording, the dead hooks in `commands.md`, and the settings entries for commands that are gone. The `docs/decisions/README.md` sync skip and the `looper …` entries stay, because looper's templates write both. `tools/nv/cmd/owners.ts:13`, `docs/agent/commands.md:620`, `.claude/settings.json:19`
- [x] The old driver's files under `.loop/` that nothing reads, and `tools/__pycache__/`, are deleted from the main checkout. `looper.toml:1`

### 5. a diagnostic code is never raced

**Does:** each area's codes live in their own module with a reserved range, and a test refuses a code
outside its module's range.

- [x] The code table is split into one module per area, each with its range. `crates/nvs-diagnostics/src/code/mod.rs:1`
- [x] A test refuses a code outside its module's range and a code used twice. `crates/nvs-diagnostics/src/code/mod.rs:146`
- [x] `conventions.md`'s diagnostic shape says how to take the next code in an area. `docs/agent/conventions.md:494`

### 6. the central tables are split per item

**Does:** a goal that adds a `Core` class, a CLI command or a feature's proof policy adds a file and
edits no shared table by hand.

- [x] The `Core` class table is one file per class, collected in one place. `crates/nvs-stdlib/src/registry.rs:1774`
  - [x] The collection is decided: a committed, rendered `crates/nvs-stdlib/src/classes.rs`, ordered by a rank each class states beside its own const.
  - [x] A `nv render` renderer writes `classes.rs` from every ranked `CoreClass` const under `crates/nvs-stdlib/src/`, and `registry.rs` re-exports it as `CLASSES`.
  - [x] Each const in today's table carries its rank, seeded from its position so the order is unchanged, and `nvs meta --json` prints the same bytes.
  - [x] The table's prose comments move into each const's own doc, rewritten for where they now stand.
  - [x] A test fails on a `CoreClass` const that has no rank and is not on a short roster of unregistered ones, such as `crate::cursor::CLASS`.
- [x] Each `nvs` command registers itself, so a new command edits no central `match`. `crates/nvs-cli/src/main.rs:144`
  - [x] The shape is decided: a module owns a `clap::Args` struct, whose doc comment is the help line, and its `run`. `main.rs` keeps one variant line and one arm line per command, as `conventions.md` § *An `nvs` command* says.
  - [x] `nvs ast` is moved, and all 113 `--help` and `-h` pages print byte for byte what they printed before.
  - [x] Every command with no subcommands is moved, and the help pages are compared again.
  - [x] Every command group, such as `service` and `queue`, moves its `Subcommand` enum into its module, and the help pages are compared again.
  - [x] `initializes`, `prepares` and the `service run` test still match each variant, and `main.rs` holds no command's fields.
- [x] `data/proofs/policy.json`'s skips are one file per feature, under `data/proofs/skip/`. `tools/nv/schema/proofs.ts:51`

### 7. a generated file never conflicts in a merge

**Does:** every file a tool writes is declared generated, and a merge renders it again.

- [x] `.gitattributes` marks every rendered file generated, and the perf ledger `members.ndjson` as `merge=union`. `.gitattributes:43`
- [x] `commands.md` says how a person resolves a merge conflict in a generated file: take either side, then render. `docs/agent/commands.md:1`
- [x] Once looper's goal `landing-fixes` is reached, `looper.toml`'s `[merge]` table names every rendered file and the commands that render them. If it is not reached when this item comes up, the session writes `BLOCKED looper's goal landing-fixes is not reached`, because only the user starts looper's loop. `looper.toml:1`

## Handoff

### State

Every stage is done and the goal-end gates are green. The refused `DONE`'s `floor-debt` run left
six proof programs red, and one cause made five of them red:

- The five PDF attacks that declare `ends-early ... FATAL` ran to their last line under a recorded
  run, which sets `NOVIS_NO_FILE_CACHE`. A built-in component compiled cold, inside the first
  request that calls it, allocates on wasmtime's worker threads and frees on the request's thread.
  So the request's balance fell by about 360 MB, and its ceiling bought it that much more. A warm
  run deserializes the component and was right. `built_in` in `crates/nvs-cli/src/extensions.rs`
  now compiles inside a `budget::Detached` bracket, and
  `a_cold_compile_of_a_built_in_component_buys_its_request_no_memory` pins it. The attacks are
  unchanged and green both ways.
- The `Core\Ldap\Entry::string` attack was red in 162 ms and is green alone and recorded. It needs
  the `samba-ad` fixture, which was not up for that run. Not checked further.

Nightly run 39108526796 is still the newest, and the last handoff accounted for every job in it.

### Next

The goal end: the driver judges `floor-debt` on the `DONE` claim.
- [x] The cold-compile ceiling fix. `crates/nvs-cli/src/extensions.rs:142`
- [ ] `floor-debt` green, then `DONE`. `tools/nv/cmd/debt.ts:1`

### Backlog

- Moved whole to goal `works-without-looper`'s backlog. `docs/goals/works-without-looper.md`
