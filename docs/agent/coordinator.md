# Agent coordinator loop

Novis is built by a long series of unattended work sessions. This file describes how that loop is driven
and why it is shaped the way it is. The loop itself is looper, an external tool: `looper.toml` at the root
is this repository's configuration, and looper's own design document is how it works. This file holds only
what is specific to Novis.

## The rule that determines the design

**A loop that lives inside one conversation cannot run hundreds of iterations.** Every iteration appends
permanently to that conversation — the subagent prompt, its report, the re-read handoff — so a
coordinator session is soon either stopped or auto-summarized, and a summarized coordinator quietly loses
the standing instructions it exists to enforce. No prompt wording fixes this; a conversation cannot forget.

So **the loop does not live in a conversation.** looper starts one *fresh* `claude` session per
iteration. Context resets to zero every time, so the last session costs what the first did. The loop's
memory lives on disk: git history, and the live goal's file under `docs/goals/` with its `## Handoff`.

Four things follow, and they are the whole design:

1. **The per-session prompt is fixed.** `looper.toml` sets no `[prompts]`, so every session gets looper's
   built-in prompt. Every rule a session needs on top of it is in [AGENTS.md](../../AGENTS.md), which the
   harness inlines, so nothing a coordinator writes can drift.
2. **Stop conditions are script, not judgment.** looper decides when to stop, from observable facts — an
   acceptance check's exit code, whether `HEAD` advanced, the status line a session wrote to disk.
   Nothing on the stop path depends on a model's self-assessment.
3. **The handoff is bounded.** The live goal's `## Handoff` is read in full by every future session, so
   each session overwrites it rather than appending to it, and it aims at about 60 lines. Its shape is
   [conventions.md](conventions.md) § *A handoff*. The target is guidance for the session writing it, and nothing
   measures it.
4. **State and knowledge are separate files.** The handoff is rewritten every session;
   [playbook.md](playbook.md) grows a bullet at a time and is otherwise left alone, so no session pays to
   regenerate — and quietly reword — the permanent half.

## Files

| File | Role |
|---|---|
| `looper.toml` | This repository's looper configuration: the goals directory, the playbook, the context budget, the verify step, the prose scope and the commit rules. |
| `docs/goals/README.md` | The chain. Its front matter's `chain:` list **is** the order, and its first slug is the live goal. `looper render` writes the body. |
| `docs/goals/<slug>.md` | A goal: its stages, its acceptance checks, its `context` list, its standing decisions and its `## Handoff`. Edited in place or through `looper goal`; `looper lint` checks it. |
| [loop-authoring.md](loop-authoring.md) | How a *new* goal is written: measure first, **scope the context**, what makes one drivable, what to pre-authorize, the stage order. |
| [playbook.md](playbook.md) | The traps a session paid for once, one file per bullet under `docs/agent/playbook/`. |
| [conventions.md](conventions.md) | The shape of everything the repo writes, so no session re-derives it from an existing example. |
| `.loop/status.txt` | One line written by each session: `CONTINUE …`, `DONE …` or `BLOCKED …`. |

`.loop/` is gitignored in full.

## The acceptance test

A goal's acceptance checks are in its front matter, each tied to the stage that owes it. Every check is
an exit code plus an exact or ordered match on real output, so no model judgment sits anywhere on the
stop path. The fields a check takes are looper's `docs/goal-format.md`. When a session writes `DONE`, looper
runs the goal's checks, and the machine outranks the claim: a red check keeps the goal open.

Inspect a goal's checks with `looper goal show`, and run one by hand with `looper check --name <text>`.
An agent's own check is `bun nv affected --run`, which runs what its change reaches.

**A reached goal is deleted, not harvested.** `looper.toml` sets `harvest = false` and
`carry_to_floor = false`, because a goal's decisions are already rules by the time it is reached. So
the session that writes `DONE` moves each decision only the goal's prose holds to a rule or a module doc
first, and turns each check that proves more than a suite does into a test (AGENTS.md § *In the loop*).

## Running it

    looper loop add
    looper loop start

That is the whole of it. The run walks the chain until it is walked, something goes wrong, or you stop it.
`looper status` shows every loop, `looper log` a session's events, and the controls are `looper loop
stop`, `pause`, `resume`, `freeze`, `thaw`, `say` and `interrupt` (`looper loop --help`).

A **side goal** runs in a lane: `looper loop add --side <slug>` adds its loop, looper makes the worktree
`.agent-tmp/worktrees/lane-<n>` on branch `side/<slug>`, and `looper.toml`'s `before_run` hook, `bun nv
side --prepare --hook`, readies it before each run. A side loop whose `.agent-tmp/worktrees/side/<slug>`
already exists keeps that worktree, and `bun nv side --prepare <slug>` makes and readies one by hand.
[commands.md](commands.md) § *The loop's hooks, one command each* is what a side goal is and how it
lands.

**A session runs under `--permission-mode auto`, and that is not an incidental setting.** A `claude -p`
session auto-denies any tool call that would otherwise prompt, so under `default` an unattended session
cannot run `cargo`, cannot commit, and cannot write its own handoff. Under `auto`, reads and edits inside
the tree run unasked, a call that matches `.claude/settings.json`'s allowlist runs unasked, and every other
call is reviewed by the harness's classifier, which denies the risky ones. A denied call does not stop
the session; it comes back to it as an error. The allowlist is therefore the set of commands a session
runs routinely. Broad rules — `Bash(*)`, an interpreter, `Agent` — are dropped by the harness in this
mode, so a command added there is added narrowly or not at all. `bun nv allow --learn` proposes the rules
a session's shell calls owed. The tree must be trusted ([setup.md](../setup.md), *Machine-local harness
settings*), or the allowlist is ignored and every such call waits on the classifier.

### The loop's own machinery is a person's to change

The loop never spends a session on itself. A change to it is made by a person, between runs or while the
run is paused, and is judged the way any change to `tools/` is: `looper orient` must still produce a pack,
`looper lint` must pass, and `nv verify` must be green.

## Setting a new goal

**[loop-authoring.md](loop-authoring.md) owns this, in full** — measuring before you write anything, what
makes a goal drivable at all, what must be pre-authorized so the run never halts on `BLOCKED`, the stage
order, and how to write the item list so groups fall out of it.

## Why not a coordinator conversation at all?

One is still fine for a **short** run — a dozen sessions, where you want to watch and intervene. Give each
subagent the goal's pack from `looper orient`, one subagent at a time (they share one working tree and
one handoff; concurrent sessions race). Keep the coordinator's own context small: never re-read a diff,
never re-derive the plan, never second-guess verified work. But restart it before it nears the
interactive ceiling of AGENTS.md § *The context budget* — and understand that restarting is not a
workaround for the context problem, it *is* the loop's design, just done by hand.

**Could a parallel `Core`-domain lane work?** Partly. A new `Core` class is one line in
`nvs_stdlib::registry`, one in `nvs_stdlib`'s `symbols`, and its own new module, so two sessions adding
two domains do not touch the same lines, and the checker and end-to-end tests are one file per rule area
and per feature area. The chain run itself is still **one working tree and one handoff**. A second lane is a **side goal**, as
§ *Running it* says. What a side goal does not do is split the chain: it is written by hand for work that
can run beside the chain, and the chain still walks one goal at a time.

**Neither is the first lever, and the reason is the context ceiling.** A session's fixed cost —
orientation before the first edit, then verify, docs and commit after the last one — argues for putting
several related slices in one session. But an agent starts missing what it has already read well before
its window is full, so the saving is in reading less per session, not doing more. AGENTS.md § *The
context budget* is the gate.

**Reading less is a mechanism rather than an instruction.** `looper orient` prints a session's whole step
1 out of the goal's own `context` list, so a session does not pay for the unscoped orientation to start.
What a goal narrows to is decided once, when the goal is written ([loop-authoring.md](loop-authoring.md)
§ 2), from what the last run's sessions read rather than from taste.

Parallel lanes are the *second* lever, and side goals are their form: a worktree and a handoff per lane,
each started by hand and merged when green. They do not care about the context ceiling, since each lane
is its own session; what they spend is a second checkout and a second `target/` for as long as the side
goal's worktree lives.
