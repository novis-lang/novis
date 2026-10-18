---
looper: 1
title: The project works the same without looper
summary: "Every check a change must pass is written once in bun nv, runs without looper, and is the code looper runs for it, and the checks below pass."
context: [AGENTS.md, looper.toml, docs/agent/commands.md, .claude/settings.json, tools/git-hooks/commit-msg, tools/nv/main.ts]
stages:
  - id: 1
    title: nv owns every check looper has
    status: done
    context: [docs/agent/prose.md, tools/nv/cmd/guard.ts]
  - id: 2
    title: looper runs the project's command for each check
    status: done
checks:
  - id: lint
    stage: 1
    name: "goals, chain, README and playbook lint clean"
    run: [looper, lint]
  - id: without-looper
    stage: 1
    name: "the guard, the prose check and the commit hook give the same verdicts with no looper on PATH"
    run: [bun, test, tools/nv/test/without-looper.test.ts]
  - id: doctor
    stage: 2
    name: "looper finds no drift in the project's config"
    run: [looper, doctor]
  - id: doc-links
    stage: 2
    name: every doc link and anchor resolves
    run: [bun, nv, verify, --doc]
  - id: owners-closes
    stage: 2
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, works-without-looper]
    reach: true
---

# The project works the same without looper

## Why

Only a few developers have looper, and CI never does. Goal `lean-loop` moved several checks into
looper: `nv guard` went in `d4671aab6`, and `nv bg` with `verify --start` and `--wait` in `9571a869a`.
Markdown prose shape and commit-message shape were never in `nv` at all. So today a developer or an
agent without looper skips the guard, the prose check and the commit shape check, and
`commands.md` § *Without looper* tells them to follow those rules by hand.

## Standing decisions

**The user decided:**
- **The project runs and is developed the same way with and without looper.** Every check a change
  must pass runs with `bun nv` alone, and gives the same verdict both ways.
- **This is a base rule, AGENTS.md § *Novis works without looper*,** for every goal and not this one
  alone. Nothing is ever moved, replaced or deleted so that the work needs looper, because only a few
  developers have it and CI never does.
- **Each check is written once, in `bun nv`, and looper runs the project's command for it.** Looper
  gets the update that lets `looper.toml` name that command. Its own implementation stays for the
  projects that set none. Looper's goal `project-gates` owns that update.
- `nv links` and `nv layout` stay, locally and in CI. They check more than `looper links` and
  `looper layout`: source-file links and their form, crate and bench-package rows, and the
  `[audited unsafe]` marker.

**These calls were the agent's, for the user to overturn:**
- A check's configuration moves to `nv` with the check, unless looper itself still reads it. So
  `[[guard.deny]]` and `[prose] paths` moved into `nv guard` and `nv prose`. `[commits]` stays in
  `looper.toml`, because `looper wrap` reads it too, and the commit hook reads it there with awk.
  `[prose] scope` stays, because it is the words looper writes into `docs/agent/prose.md`.
- The settings keep two `PreToolUse` entries, because `looper doctor` and `looper sync` want one whose
  command is exactly `looper guard`. That entry keeps looper's loop-only parts (edit claims and the
  pending entries `looper exec` reads) and runs `bun nv guard --gate` through `[gates] guard`. The
  other is `command -v looper >/dev/null 2>&1 || bun nv guard`, so the rules run once either way.
  The generic rules looper has built in (shell writes, sleeps, kills by name, a build piped into
  `head`, a wide test run, a whole read of a big file) are `nv guard`'s in this repository.
- `tools/git-hooks/commit-msg` checks the shape itself and no longer runs `looper hook commit-msg`.
- Background verify is not restored. A harness's `run_in_background` runs `bun nv verify`, and
  joining another session's run is loop coordination, which needs looper anyway.
- Goal lint and the goals README render stay looper's. Goals are the loop's schedule, and a
  developer without looper edits a goal by hand, as `commands.md` says.

**No product behaviour changes.** Only `tools/`, the hooks, the settings and the agent docs change.

**Trade-offs.**
- The guard starts `bun` on every tool call, where `looper guard` is one native binary. With looper,
  `looper guard` starts it for `Edit` and `Write` too, because its entry matches them. Without
  looper, it starts only for `Read` and `Bash`.
- The commit hook's shape check stays in sh and awk, as its trailer check is, so a commit starts no
  new process.
- Looper keeps two implementations of each gate: its own and the project's command. Novis uses
  one.

## Stages

### 1. nv owns every check looper has

**Does:** every gate a change must pass runs without looper, with its rules in one home.

- [x] `bun nv guard` holds looper's generic built-in rules and the four `[[guard.deny]]` rules, which move into it. `d4671aab6^:tools/nv/cmd/guard.ts` is the starting point. `looper.toml:85`
- [x] The `PreToolUse` hooks run `nv guard`'s rules once: through `looper guard` where looper is installed, and `bun nv guard` where it is not. `.claude/settings.json:10`
- [x] `bun nv prose` checks the shape of the touched Markdown in `[prose] paths`: a sentence over 25 words, a dash that joins two sentences, and a banned word. `docs/agent/prose.md:5`, `looper.toml:106`
- [x] `tools/git-hooks/commit-msg` checks the subject's shape itself, from the type list and length `[commits]` holds now. `tools/git-hooks/commit-msg:89`, `looper.toml:124`
- [x] `tools/nv/test/without-looper.test.ts` runs the three with no looper on PATH and pins their verdicts.
- [x] `commands.md` § *Without looper* gives an `nv` command for every gate, and names the rest as loop-only. `docs/agent/commands.md:837`

### 2. looper runs the project's command for each check

**Does:** `looper links`, `looper layout`, `looper prose` and `looper guard` run `nv`'s code in this
repository, so both ways give one verdict.

- [x] Once looper's goal `project-gates` is reached, `looper.toml` names the `bun nv` command for each gate. If it is not reached when this item comes up, the session writes `BLOCKED looper's goal project-gates is not reached`, because only the user starts looper's loop. `looper.toml:1`
- [x] AGENTS.md and the agent docs name the `bun nv` command for each gate, and `looper …` only for loop work. `AGENTS.md:1`, `docs/agent/prose.md:5`

## Handoff

### State

Both stages are done, and the goal is reached. `docs/agent/prose.md` and `docs/agent/testing.md` are on
`looper.toml`'s `[sync] skip` and name `bun nv prose` and `bun nv affected`. `[prose] scope` stays,
because AGENTS.md's managed block renders it. This file's decisions are in `docs/agent/commands.md`
§ *Without looper*, and its open backlog moved to `release-hardening`'s. `looper doctor`,
`bun nv verify --doc` and `bun nv owners --closes works-without-looper` pass.

The nightly CI run's reds are all on `1a78b49ef`, the pushed `main`. Local `main` already fixes the
render, supply-chain, attribution and `docs/novis.md` jobs (`a9fdc1a97`, `8f8966275`), and the perf
debt and cost guards are `release-hardening`'s. Pushing is the user's.

### Next

Nothing: the driver retires this goal, and `release-hardening` is live next.

### Backlog

- The rest moved to `release-hardening`'s backlog.
- Two looper gaps. Its `links` misses `<a id>` anchors, a skip for generated pages and a code span that wraps onto the next line, which looper's goal `project-gates` notes. And `looper guard` matched a commit in looper's own checkout against a claim on this tree's `docs/goals/README.md`, which no looper goal owns yet (`crates/looper-core/src/claims.rs` in looper's repository).
