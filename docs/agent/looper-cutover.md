# Moving the loop onto looper: the cutover runbook

The cutover replaces Novis's own loop driver with looper, the external loop tool, and deletes the
driver. It runs without a person, in two goals. Goal `looper-cutover`
([goals/looper-cutover.md](goals/looper-cutover.md)) runs under the old driver, does § 0 to § 6 and § 8,
and deletes nothing the driver loads. Once it is reached, the driver hands the run to looper itself.
Then `nv-loop-retired`, the goal it writes first on looper's chain, runs under looper and does § 7's
proof and deletes. This file is the spec both goals follow, and the goal file holds the calls
this file leaves open. It starts when both halves of the preparation are reached:

- **The Novis half**: the chain goal that moves the shared code out of `tools/nv/driver/` and splits
  every hook out as a `bun nv` command. It is reached when its slug is gone from `data/chain.json` and
  `bun test ./tools/nv/test/driver-boundary.test.ts` passes whole.
- **The looper half**: looper's goals for project prompts and `[orient] extra`, wrap extension
  sections and checks, the after-session hook and hook timeouts, side loops, the playbook directory
  and `until` trailers, sync opt-outs, and the `nv` importer. The installed looper has all of them
  when the `waitFor` gate in goal `looper-cutover`'s record opens, and the goal file's § *Why here*
  says why that gate cannot open early.

Looper's `docs/upgrade-guide.md`, Part 1 and Part 3, is the general procedure. This file holds only what
is specific to Novis, in the order the steps run. A step here that contradicts the guide wins for Novis.

## The user's calls, which bind every step

- **Nothing of the loop is lost**: not the main chain, not a side goal, not a check condition, not a
  playbook trailer. A gap is closed by a general looper feature, never by a fork of looper and never by
  a Novis-only field in looper's goal format. A gap found during the cutover stops it, and the user
  hears about it.
- **Commit subjects stay long.** `looper.toml` sets the subject limit to 120, and the type list holds
  every type [conventions.md](conventions.md) § *A commit message* names, plus `chore` for the guide's
  own commits.
- **The ceilings and the prose scope stay Novis's**: 200k for a loop session, 250k for an interactive
  session and each subagent, the 120k slice gate, all at AGENTS.md § *Session workflow* step 2. The
  prose limits bind only text an end user reads, and agent-only docs are exempt. `looper.toml` points
  the shared `agents` block at these values.
- **Decision records stay retired.** `[sync] skip` holds looper's `docs/decisions/README.md` template,
  and its record lint is off. Part 3's step that reshapes old records is not run.

## 0. Before anything moves

1. `looper --version` runs and `looper status` exits 0.
2. **The old driver runs this, on `main`.** Its run is the one live run in the main tree, and its
   sessions commit each slice on `main`. No session deletes, or changes what it does, any file the
   driver loads. The side worktrees are left alone (§ 6).
3. Nothing is saved to compare against: the first session under looper is § 7's proof.

## 1. Register, then fix each guess

`looper project add .` writes `looper.toml`. Then, against what it guessed:

- `[project]`: `scratch = ".agent-tmp"`, `stacks = ["rust"]`, no `harvest` (Novis harvests nothing),
  and no carry to the floor: no check moves to the floor when its goal is reached
  (`tools/nv/lib/chain.ts:6-8` is that rule today).
- The commit-msg subject limit (120) and the type list, as above. `tools/git-hooks/` is the hooks
  path, and the hook passes a commit with the trailer check alone when `looper` is not on `PATH`:
  contributors to this public repository may not have looper.
- `[loop] min_free_gb` takes the driver's disk gate. `[budget]` takes its hard and kill caps. The
  soft cap stays in AGENTS.md.
- The check timeout default is 1800 s, which is the driver's.

## 2. Import

`looper import --from nv --dry-run`, then `looper import --from nv`, `looper render`, `looper lint`. The
import is right when:

- Every chain goal and both side goals (`data/goals/side/`) are goal files, the side goals off the
  chain, each with its current handoff and every recorded stage state.
- A `cargo-named` check runs `cargo test --test <name>` or the project's test runner, never with `-p`
  (AGENTS.md rule 5).
- A `nvs-suite` check runs `bun nv suite <tree>... [--case <path>]... [--min-passing <n>]`, so its
  case conditions are kept.
- A `want` is a matcher over stdout and stderr together. Many Novis checks print their wanted text on
  stderr.
- A goal's `env` (Docker services, WSL target, valgrind skip list) and its `rules`, `adrs`, `shapes`,
  `plan` and `milestones` selectors arrive where looper's hooks and `[orient] extra` can read them.
- The playbook arrives in looper's directory form, one file per bullet, with every
  `until: gone|exists path:needle` trailer.
- No `TODO` is left in any check. `looper lint` refuses one.

## 3. The hooks and checks

Each row is one entry in `looper.toml`. The commands exist before the cutover and each has its own
tests, so this step only wires them.

| looper | Novis command |
|---|---|
| `[verify] run` | `bun nv verify` |
| `[hooks] before_session` | `bun nv services --up [--goal <slug>]` |
| `[hooks] after_session`, with `on_fail = "warn"` | `bun nv sweep [--goal <slug>] [--done] [--keep-runs <n>] [--session <label>]`, then `bun nv allow --learn <the session's event log>` |
| the side loop's preparation hook | `bun nv side --prepare <slug> --dir <the worktree> [--min-free-gb <n>]` |
| a Linux-leg floor check, or `before_session` | `bun nv legs [--goal <slug>] [--full]` |
| a `nvs-suite` check | `bun nv suite <tree>... [--case <path>]... [--min-passing <n>] [--full]` |
| the bench turn, on the cadence `bun nv sweep` keeps | `bun nv benches [--owed]` |
| `[orient] extra` | `bun nv pack --goal <slug> --stage <n>` |
| `[wrap] sections` `plan`, `plan-edit`, `milestone` | `bun nv plan --apply - --dry` to check, `bun nv plan --apply -` to write |
| `[wrap] checks` | `bun nv reference --check`, and the gates `bun nv session --wrap` runs before its commits today |
| `[[guard.deny]]` | each Novis rule of `bun nv guard` (`tools/nv/cmd/guard.ts`) that the built-in guard does not already hold |

The doc gate and the owner gate become floor checks. `bun nv sweep` writes `.loop/last-sweep.md`, which
looper's orient prints, and exits 0, so a red sweep never pauses the run. `bun nv pack` and the hooks
take the goal and the stage from `--goal` and `--stage`, then from `LOOPER_GOAL` and `LOOPER_STAGE`.
Where looper exports other names, the argv passes flags.

## 4. The rules and the harness

- **AGENTS.md**: the loop and goal parts become the guide's *Goals and the loop* block — `looper orient
  --full`, `looper verify`, `looper wrap <file>`. That covers § *Where to look*'s first row, rules 4 and 6,
  § *Session workflow* and the chain bullets in § *Keep each slice small*. Every domain rule stays.
- **`.claude/settings.json`** is the user's to approve, so no session edits it. It gains
  `Bash(looper:*)`, `PowerShell(looper:*)` and a PreToolUse hook that runs `looper guard`, beside the
  `bun nv guard` hook, which stays. The driver's handover adds and commits these before it starts
  looper (§ 7 step 2, `tools/nv/driver/handover.ts`), because a session under looper needs them.
  It loses the `bun nv` entries for `chain`, `goal`, `loop`, `loop-stats`, `orient`, `playbook` and
  `session`, Bash at `:15`, `:27`, `:33`, `:34`, `:36`, `:41`, `:53` and PowerShell at `:98`, `:110`,
  `:116`, `:117`, `:119`, `:124`, `:136`: `nv-loop-retired`'s handoff names that change for the user.

## 5. What reads the old goal data, and moves with it

Each item is re-pointed at looper's goal files, or deleted with what it serves. Goal `looper-cutover`
does the part that leaves the old driver working: a reader outside the driver's code learns to work
when `data/chain.json` is gone, and behaves as before while it is there. `nv-loop-retired` does the
rest.

- **`tools/nv/lib/chain.ts`**, the one reader of goals and the chain for the tools that stay:
  `cmd/affected.ts:47`, `cmd/select.ts:30`, `cmd/plan.ts:12`, `cmd/owners.ts:46`, `cmd/holes.ts:35`,
  `cmd/audit.ts:27`, `select/full.ts:35`, `select/seed.ts:23`. The new commands read goals only
  through it, so re-pointing it re-points them.
- **`bun nv plan --check`** compares the milestone table with the chain (`cmd/plan.ts:286-414`), and CI
  runs it (`.github/workflows/ci.yml:933`).
- **`bun nv render --check`** covers `docs/agent/goal-plan.md` through `renderers/goal-plan.ts`
  (registered at `renderers/index.ts:15`, CI `ci.yml:917`). The renderer and the page are deleted.
- **`bun nv check`** validates the `chain`, `goal`, `handoff` and `playbook` records
  (`schema/index.ts:10-17`, CI `ci.yml:915`). Those schemas are deleted with their data.
- **CI** runs `bun nv chain --check` (`ci.yml:939`) and `bun nv playbook --check` (`:941`). Both become
  `looper lint`, and the comment at `:1116` names `bun nv loop`.
- **`crates/nvs-stdlib/tests/spec_registry_coverage.rs:390`, `:412` and `:546`** read
  `data/chain.json` and panic without it.
- **Tests**: `tools/nv/test/render.test.ts:5` and `:90-98` read the chain;
  `tools/nv/test/registers-reads.test.ts:29-31` reads `data/playbook/`.
- **`cmd/affected.ts:57` and `:119-130`** take the session's base commit from
  `.loop/session-start.json` when `NOVIS_LOOP_WRITES` is set. They take it from looper's session base
  instead, and `LOOPER_WRITTEN` says a loop session is running.
- **`cmd/guard.ts`**: the `goal-grep` rule (`:370-388`, `commands: ["loop"]`, `GOAL_CHECKS` at `:319`)
  and the `loop --goal-only|--run` refusal (`:545`) move to looper's goal paths and commands.
  `nv guard --check` (`:608-639`) fails on a rule that names a command `bun nv help` no longer lists.
- **Markdown links** into the deleted files: `AGENTS.md`, `CONTRIBUTING.md`, `docs/setup.md` and the
  files under `docs/agent/` link to `coordinator.md`, `session-prompt.md`, `loop-authoring.md`,
  `playbook.md`, `goals/` and `tools/nv/cmd/playbook.ts`. `bun nv links` names every one once the files
  are gone.

## 6. Side goals and their worktrees

`compare-page` and `search-questions` are side goals with live worktrees at
`.agent-tmp/worktrees/side/compare-page` and `.agent-tmp/worktrees/side/search-questions`, on branches
`side/compare-page` and `side/search-questions`. `compare-page`'s worktree has its own `.loop/`.

- **The cutover never touches them**: not their worktrees, not their branches, not their `.loop/`.
  Each branch keeps the driver it was cut with, so a run there goes on working.
- Each side goal is imported off the chain (§ 2). Under looper its run reuses its existing worktree and
  branch, and on green it stops with the landing steps instead of retiring.
- A side goal that lands after the cutover is rebased onto a `main` without the driver. Its landing is
  run with `looper`, not `bun nv loop`.

## 7. Prove it, then delete

1. `looper lint` is green, and `looper sync --check` exits 0. Goal `looper-cutover`'s checks hold this.
2. The driver's handover deletes `data/chain.json` and `looper-cutover`'s files, commits them as
   `chore(goals): the loop moves onto looper`, adds § 4's looper entries to `.claude/settings.json`
   and commits that path alone, and runs `looper loop start`.
3. The first session under looper, `nv-loop-retired`'s first stage, is the proof: its wrap commits,
   including a `## plan:` section, and its status line is read. `bun nv sweep` wrote
   `.loop/last-sweep.md` or found nothing red. What fails here is fixed before anything is deleted.

Only then, and as the last change, `nv-loop-retired` deletes: `tools/nv/cmd/{loop,chain,goal,session,orient,loop-stats,playbook}.ts`
and their `main.ts` entries, `tools/nv/driver/` with `tools/nv/driver/test/`, `data/chain.json`,
`data/goals/`, `data/playbook/`, `docs/agent/goals/`, `docs/agent/goal-plan.md`,
`renderers/goal-plan.ts`, the four schemas of § 5, and `docs/agent/{session,repair}-prompt.md` once every
rule they hold is in AGENTS.md. `docs/agent/coordinator.md`, `loop-authoring.md` and `playbook.md`
describe the driver, and the goal file's § *Standing decisions* says what becomes of each. Then
`lib/written.ts` stops writing `NOVIS_LOOP_WRITES`, and the old driver's files are deleted from the
main tree's `.loop/`, with its `.gitignore` line kept. Looper's own files there stay.

The commits are the guide's three: `chore(goals): import goals into the looper format` in goal
`looper-cutover`, then `chore(tools): drop the project loop driver for looper` and `docs(agent): point
the loop rules at looper` in `nv-loop-retired`.

## 8. The first sync

Part 3 of the guide, with these differences:

- `docs/agent/commands.md` is Novis's own and is not replaced: `[sync] skip` holds it, as it holds the
  decision-record template.
- The `.gitignore` and `.gitattributes` blocks go at the top of each file. At the end they override
  Novis's `**/fixtures/crlf/** -text` and its `!.vscode/settings.shared.json`.
- `docs/agent/rust.md` and `docs/agent/bun.md` state defaults that Novis departs from: crates that
  deny `unsafe` in their own `[lints]`, the unsafe modules, and caret dependency ranges. Novis's rules
  win, and the difference is said once in AGENTS.md.
- No decision record is reshaped.
- `looper doctor` is green, which includes `./target/debug/nvs.exe` counting as the `nvs` an allow
  entry names. It fails until `.claude/settings.json` has the `looper guard` hook, which the handover
  writes, so it is a check of `nv-loop-retired`'s first stage and not of goal `looper-cutover`.
