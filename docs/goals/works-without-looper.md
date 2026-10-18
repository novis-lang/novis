---
looper: 1
title: The project works the same without looper
summary: "Every check a change must pass is written once in bun nv, runs without looper, and is the code looper runs for it, and the checks below pass."
context: [AGENTS.md, looper.toml, docs/agent/commands.md, .claude/settings.json, tools/git-hooks/commit-msg, tools/nv/main.ts]
stages:
  - id: 1
    title: nv owns every check looper has
    status: open
    context: [docs/agent/prose.md, tools/nv/test/guard-deny.test.ts]
  - id: 2
    title: looper runs the project's command for each check
    status: open
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
- **Each check is written once, in `bun nv`, and looper runs the project's command for it.** Looper
  gets the update that lets `looper.toml` name that command. Its own implementation stays for the
  projects that set none. Looper's goal `project-gates` owns that update.
- `nv links` and `nv layout` stay, locally and in CI. They check more than `looper links` and
  `looper layout`: source-file links and their form, crate and bench-package rows, and the
  `[audited unsafe]` marker.

**These calls were the agent's, for the user to overturn:**
- A check's configuration moves to `nv` with the check. `looper.toml` keeps only the command looper
  runs. So `[[guard.deny]]`, `[prose]` and `[commits]` move out of `looper.toml`.
- The settings' `PreToolUse` hook is a small script. With looper on PATH it runs `looper guard`,
  which keeps its loop-only parts (edit claims and the pending entries `looper exec` reads) and runs
  `bun nv guard` for the rules. Without looper it runs `bun nv guard`. The generic rules looper has
  built in (shell writes, sleeps, kills by name, a build piped into `head`, a wide test run, a whole
  read of a big file) are `nv guard`'s in this repository.
- `tools/git-hooks/commit-msg` checks the shape itself and no longer runs `looper hook commit-msg`.
- Background verify is not restored. A harness's `run_in_background` runs `bun nv verify`, and
  joining another session's run is loop coordination, which needs looper anyway.
- Goal lint and the goals README render stay looper's. Goals are the loop's schedule, and a
  developer without looper edits a goal by hand, as `commands.md` says.

**No product behaviour changes.** Only `tools/`, the hooks, the settings and the agent docs change.

**Trade-offs.**
- The guard starts `bun` on every tool call, where `looper guard` is one native binary. With looper,
  a call starts both. `nv guard` ran as this hook before `d4671aab6`, so the cost was paid then.
  Stage 1 measures it once.
- The commit hook's shape check stays in sh and awk, as its trailer check is, so a commit starts no
  new process.
- Looper keeps two implementations of each gate: its own and the project's command. Novis uses
  one.

## Stages

### 1. nv owns every check looper has

**Does:** every gate a change must pass runs without looper, with its rules in one home.

- [ ] `bun nv guard` holds looper's generic built-in rules and the four `[[guard.deny]]` rules, which move into it. `d4671aab6^:tools/nv/cmd/guard.ts` is the starting point. `looper.toml:85`
- [ ] The `PreToolUse` hook is a script that runs `looper guard` where looper is installed and `bun nv guard` where it is not. `.claude/settings.json:10`
- [ ] `bun nv prose` checks the shape of the touched Markdown in `[prose] paths`: a sentence over 25 words, a dash that joins two sentences, and a banned word. `docs/agent/prose.md:5`, `looper.toml:106`
- [ ] `tools/git-hooks/commit-msg` checks the subject's shape itself, from the type list and length `[commits]` holds now. `tools/git-hooks/commit-msg:89`, `looper.toml:124`
- [ ] `tools/nv/test/without-looper.test.ts` runs the three with no looper on PATH and pins their verdicts.
- [ ] `commands.md` § *Without looper* gives an `nv` command for every gate, and names the rest as loop-only. `docs/agent/commands.md:837`

### 2. looper runs the project's command for each check

**Does:** `looper links`, `looper layout`, `looper prose` and `looper guard` run `nv`'s code in this
repository, so both ways give one verdict.

- [ ] Once looper's goal `project-gates` is reached, `looper.toml` names the `bun nv` command for each gate. If it is not reached when this item comes up, the session writes `BLOCKED looper's goal project-gates is not reached`, because only the user starts looper's loop. `looper.toml:1`
- [ ] AGENTS.md and the agent docs name the `bun nv` command for each gate, and `looper …` only for loop work. `AGENTS.md:1`, `docs/agent/prose.md:5`

## Handoff

### State

The goal is new, and nothing is on disk for it yet. Goal `lean-loop` is live ahead of it, with only
its goal-end gates left. Looper's goal `project-gates` is written in looper's repository.

### Next

Stage 1, on `tools/nv/main.ts`, `.claude/settings.json` and `looper.toml`.
- [ ] `bun nv guard` comes back, from `d4671aab6^:tools/nv/cmd/guard.ts`, with the deny rules moved out of `looper.toml`, and the hook script picks it or `looper guard`. `.claude/settings.json:10`, `looper.toml:85`
- [ ] `bun nv prose` for the Markdown in `[prose] paths`. `looper.toml:106`
- [ ] The commit hook checks the subject's shape without looper. `tools/git-hooks/commit-msg:89`

### Backlog

- Looper's `links` misses three things another project may need: `<a id>` anchors, a skip for generated pages, and a code span that wraps onto the next line. Looper's goal `project-gates` notes them.
