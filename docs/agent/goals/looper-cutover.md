---
position: last
---
# Loop goal — Novis's goals, checks, hooks and rules are in looper's format, and the next goal on looper's chain deletes the old driver

The loop moves onto looper, the external loop tool, without a person. This goal is the move, run as
loop sessions under the driver it replaces. When it is reached:

- **`looper.toml` holds the user's calls**, every hook and check is wired to its `bun nv` command, and
  `looper lint` and `looper sync --check` exit 0. `looper doctor` is not judged here: it needs the
  `looper guard` hook, which only the handover writes, so it is a check of `nv-loop-retired`'s first
  stage.
- **Every goal still on the chain and both side goals are goal files in looper's format**, with
  `nv-loop-retired` first on looper's chain. `data/chain.json` names this goal alone.
- **Nothing the running driver loads is deleted or changed in what it does.** Deleting the driver is
  `nv-loop-retired`'s work, under looper.

Then the driver itself hands over (`handover: "looper"`, `tools/nv/driver/handover.ts`): it deletes this
goal's record, prose and handoff and `data/chain.json`, commits them, adds `Bash(looper:*)`,
`PowerShell(looper:*)` and the `looper guard` hook to `.claude/settings.json` and commits that, then
starts looper's main loop and ends its own run for good.

**[looper-cutover.md](../looper-cutover.md) is this goal's spec.** It holds every step's detail, and
this file names which stage does which of its sections and the calls the runbook leaves open.

## Why here

It goes directly behind `looper-ready`, which turns every hook looper will call into a `bun nv`
command. It cannot start earlier, and nothing after it on today's chain runs under the old driver. Its
record's `waitFor` keeps it from starting before the installed looper can import Novis without loss:
`looper import --from nv --dry-run` must exit 0, name `docs/goals/compare-page.md`, and print no line
with `dropped` or `lost`. Side goals arrive in the first stage of looper's `nv-import-fidelity`, and
its later stages remove the last loss note (its test
`import_nv_of_the_whole_fixture_prints_no_loss_note`), so the gate opens only once the whole importer
is installed. The import also reads this goal's own record, so the gate stays shut while the importer
refuses its `waitFor` and `handover` fields.

It carries `position: last` because every goal on the chain is pinned there.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. `bun nv verify` stays green, and `bun nv loop`
keeps working at every commit: it is the program that runs this goal.

## Stage 2 — register and configure

**Does:** Registers the project with looper and writes `looper.toml` with the user's calls, the guard, the budget and the disk gate.

One file set: `looper.toml` (new), `tools/git-hooks/commit-msg`, `.gitignore`.

- **The runbook's § 0 and § 1**, with the differences in § *Standing decisions*: no branch, and the
  saved sweep of § 0 step 4 is not taken.
- **The user's calls in `looper.toml`** (the runbook's header): `[commits] subject_max = 120` and the
  type list, `[budget]` with `hard` 200k for a loop session, `interactive` 250k and `slice` 120k,
  `[prose] scope = "Text an end user reads"` with `paths` on the trees AGENTS.md § *Text an end user
  reads* names, `[project] harvest = false`, `carry_to_floor = false`, `scratch = ".agent-tmp"`,
  `playbook_dir`, `[loop] sweep = "every"`, and `[sync] skip` holding `docs/decisions/README.md`,
  `docs/agent/commands.md`, `docs/agent/conventions.md` and `docs/agent/doc-style.md`.
- **Pinned by** the Stage 2 checks.

## Stage 3 — import

**Does:** Imports every goal, both side goals and the playbook into looper's format, puts `nv-loop-retired` first on looper's chain, and leaves `data/chain.json` naming this goal alone.

One file set: `docs/goals/` (new), the playbook directory, `data/goals/`, `docs/agent/goals/`,
`data/chain.json`, `docs/agent/goal-plan.md`.

- **The runbook's § 2 and § 6.** `looper import --from nv`, then `looper render` and `looper lint`.
  Each condition § 2 lists is read in the files the import wrote.
- **This goal is not imported.** Its files stay in the nv format until the handover deletes them. If
  the import writes `docs/goals/looper-cutover.md`, that file is deleted in the same slice.
- **`nv-loop-retired`** is written with `looper goal new nv-loop-retired`, filled in looper's format
  with the stages in § *What `nv-loop-retired` does*, and moved to position 1 with `looper goal move`.
- **The nv chain shrinks to this goal.** Every other goal's nv record, prose and handoff are deleted,
  because their looper goal files now hold them. `data/chain.json` keeps `looper-cutover` alone, live,
  and `bun nv render` rewrites `docs/agent/goal-plan.md`. `data/goals/side/` stays until
  `nv-loop-retired`.
- **Pinned by** the Stage 3 checks.

## Stage 4 — hooks, checks and rules

**Does:** Wires each hook and check of the runbook's § 3 into `looper.toml`, moves the loop rules in AGENTS.md onto looper's commands, and runs the first sync.

One file set: `looper.toml`, `AGENTS.md`, `docs/agent/commands.md`, `.gitignore`, `.gitattributes`,
`crates/nvs-stdlib/tests/spec_registry_coverage.rs`, `tools/nv/test/`.

- **The runbook's § 3, § 4 and § 8.** § 4's `.claude/settings.json` change is not made by a session
  (§ *Standing decisions*), and § 8's `looper doctor` is not run as a check here.
- **What reads the old goal data works without it** (the runbook's § 5). Each reader that this goal
  can change without touching what the driver does is re-pointed at looper's goal files, or made to
  work when `data/chain.json` is absent: `spec_registry_coverage.rs` and the tests under
  `tools/nv/test/` first. A reader inside the driver's own code is left as it is and listed in
  `nv-loop-retired`.
- **Pinned by** the Stage 4 checks.

## What `nv-loop-retired` does

The first goal of looper's chain, written in Stage 3, with these stages in this order:

1. **The first session under looper is the proof** (the runbook's § 7 steps 2 and 3): its wrap
   commits and its status line is read. A sweep or an orient that fails is fixed here first. This
   stage's check is `looper doctor`, which passes once the handover has written the `looper guard`
   hook.
2. **The readers.** Every reader of `data/chain.json`, `data/goals/` and `data/playbook/` the runbook's
   § 5 names is re-pointed at looper's goal files or deleted with what it serves, including
   `lib/chain.ts`, `bun nv plan --check`, `bun nv render --check`, `bun nv check`'s four schemas,
   `cmd/affected.ts`'s session base and `cmd/guard.ts`'s goal rules. CI's `chain --check` and
   `playbook --check` lines go, because CI has no looper.
3. **The deletes** (the runbook's § 7): `tools/nv/driver/`, the seven loop commands, their `main.ts`
   entries and tests, `renderers/goal-plan.ts` and `docs/agent/goal-plan.md`, `data/goals/`,
   `data/playbook/`, `docs/agent/goals/`, and the prompts once AGENTS.md holds their rules.
   `lib/written.ts` stops writing `NOVIS_LOOP_WRITES`, and the old driver's files under `.loop/` are
   deleted. Looper's own files there stay.
4. **The docs.** `docs/agent/coordinator.md`, `loop-authoring.md` and the head of `playbook.md` are
   rewritten to point at looper. Only text about the old driver itself is deleted. Then
   `docs/agent/looper-cutover.md` is deleted, and `bun nv links` is green.

Its checks: `looper doctor` exits 0 in the first stage, `bun nv verify` and `looper lint` exit 0, and
no file under `tools/` imports a deleted path.

## Standing decisions

- **The user's calls, as instructions.** Nothing of the loop is lost. A gap is closed by a general
  looper feature, never by a fork or a Novis-only field. The whole move runs without a person: ldap,
  then `looper-ready`, then this goal, then looper's chain.
- **The driver works at every commit.** No session deletes, or changes the behaviour of, any file the
  running driver loads: `tools/nv/driver/`, the seven loop commands and what they import. A reader may
  learn to work without `data/chain.json`, as long as it behaves the same while the file exists.
- **No branch.** Each slice commits on `main`, because the driver that runs this goal runs on `main`.
  The runbook's § 0 step 3 and step 4 are not run.
- **Looper's built names win.** Every key and command name here comes from looper's goal files. If
  looper's built key differs from the name here, use looper's (`looper --help`, looper's `design.md`)
  and record it in the handoff. A check whose text names such a key is changed to looper's key in the
  same slice. No other check edit is allowed.
- **A goal that left the chain before this goal ran is dropped** from the Stage 3 checks in the same
  slice, and the handoff says so. That includes a side goal that landed, whose worktree check goes too.
- **`.claude/settings.json` is not edited by a session**, neither this goal's nor `nv-loop-retired`'s.
  The driver's handover adds what looper needs (the runbook's § 4). The entries § 4 says it loses are
  written into `nv-loop-retired`'s handoff for the user.
- **The side worktrees under `.agent-tmp/worktrees/side/` are never touched**, nor their branches or
  their `.loop/`.
- **The prompts.** `[prompts]` stays unset, so looper's built-in prompts run. Each rule
  `docs/agent/session-prompt.md` holds that a looper session needs moves into AGENTS.md in Stage 4.
- **The open calls of the runbook, decided.** `coordinator.md`, `loop-authoring.md` and the head of
  `playbook.md` are rewritten to point at looper, and only text about the old driver itself is
  deleted. `docs/agent/rust.md` and `docs/agent/bun.md` follow the runbook's § 8.
- **BLOCKED only for a lost piece of the loop.** A session writes BLOCKED only when the import or a
  looper feature would lose something the runbook's header says is never lost. That is the user's
  call, and it stops the cutover. Every other open call is decided here. A step that cannot be done
  without breaking the driver goes into `nv-loop-retired` instead, and the handoff says so.
- **The tradeoffs**, as AGENTS.md asks. The language, its performance and its memory do not change.
  Agents gain looper's console, its side loops and its shared rules. They lose nothing the nv driver
  did, because each hook is the same `bun nv` code.
