# Novis autonomous work session

You are one session of an unattended loop. A driver starts a fresh session after you exit, so
**everything the next session needs must be on disk before you finish.** Nothing in your context survives.

**The driver launches you under `--permission-mode auto`.** Reads and edits in the tree run unasked, and
so does every command in `.claude/settings.json`'s allowlist. Any other call is reviewed first, and a
denied one comes back to you as an error: do not retry it in another form. Do the work with an allowed
command, or name the call you needed in the handoff. `AGENTS.md` rule 1 still holds: files are written
with Write and Edit, never through a shell.

## The six steps

`AGENTS.md` § *Session workflow* is authoritative for steps 1–5; this file adds only what the loop
changes about them, and step 6.

1. **Your orientation is already in this message — do not fetch it.** The driver piped
   `bun nv orient` in behind this prompt, narrowed to the `context` manifest in the goal's record,
   `data/goals/<slug>.json`. If it is genuinely absent, run it once and say so in the handoff — that is
   a driver bug. For something outside the goal, reach for `bun nv orient --full`, then **widen the
   goal's `context` by the selector it was missing**: `bun nv goal context --add <path>` adds a module,
   and any other field is edited in the record with Edit. A gap you only describe in the handoff is one
   the next session pays for again.
2. **Take your item, then more of the group** while `AGENTS.md` step 2's gate holds. Past 120k, stop and
   say in the handoff where you stopped.
3. **Verify with `bun nv verify`, and start it before you write the wrap.** `--start` returns at once,
   `--wait` collects the verdict, and the wrap file gets written in between. A mid-work check is `--fast`
   or `-p <crate>`, never the full gate. Add a `valgrind` run for a new refcount edge
   (`docs/agent/commands.md`).
4. **Choose the next group** while you write the handoff — you hold the context that makes that cheap.
5. **Commit one slice at a time**, each as a `## commit:` section of the wrap.
6. **Write one line to `.loop/status.txt`** (overwrite), then exit:
   - `CONTINUE <what you landed>` — the normal case.
   - `DONE <what goal was reached>` — the live goal, `docs/agent/goals/<slug>.md`, is met. Run `bun nv
     verify --doc` first and fix every broken doc link it names, then `bun nv owners --closes
     <slug>` and close or re-owner every gap it names: those are the gates a goal meets only at its
     end, and the driver does not reach the goal while one is red. Move every decision that only the
     goal's prose holds to its home, a rule or a module doc, because the goal switch deletes the prose
     (`rule:tooling/the-chain-names-its-live-goal`). A tag is not a build. A `DONE` the driver's sweep
     refuses gets a retry session, and the retry is handed the check that failed.
   - `BLOCKED <the decision only the user can make>` — a tradeoff expensive to reverse. Prefer the safe
     option and a note in the handoff; the driver **holds** the run on this, waiting for the person who
     can answer it, and carries on from the tree as it stands when they lift the hold.

**Steps 4 to 6 are one wrap file**, and the pack ends with its skeleton. **Then stop.** The driver runs
the acceptance check itself, and holds the run after `--max-stalls` sessions without a commit.

## Context is the budget, and reading is where it goes

`AGENTS.md` rules 2 and 3 say how to read. In the loop, also:

- **Do not re-read what the pack printed** — the handoff, the rules, the record sections, the shapes,
  the traps. For another record section, slice it and name it in the handoff for `[context] adrs`.
- **Delegate a read-heavy search to a subagent, and keep its answer rather than its reading.** Its
  reads are charged to its own window. Send one for "where is X, and what are its anchors" over files
  you will not otherwise open. `bun nv loop-stats --attribute` measures how much of a session reading
  takes.
- **What is safe to delegate, and what is not.** Safe: *where is X*, *how many of Y are there*, *what
  spelling does the corpus use*, over a tree you are not editing — and sent as the read-only agent
  type, `Explore`, which has no Edit or Write tool, so the boundary holds by construction rather than
  by instruction. Not safe, ever: writing anything; reading a file you are about to edit; deciding a
  design question; judging whether a check passed; the wrap. The handoff and the code must rest on
  what **you** read — a subagent's summary is a pointer to verify, not evidence to commit. There is
  no carve-out.
- **Send two or three independent searches in one message**, so they run concurrently. Independent
  means neither one's prompt depends on the other's answer.
- **A delegated search gets the question, the place to look and the answer's shape — never the
  project.** It starts with its own copy of `AGENTS.md`. Ask for `file.rs:NN` anchors and one line
  each, never excerpts. The whole prompt:

      Search only under crates/nvs-stdlib/src. Find every CoreMethod row whose return type is
      CoreTy::Instance. Return one line each: `file.rs:NN  Class::member  -> instance name`.
      No excerpts, no commentary. If you find none, say so.

## The handoff's shape

The handoff is the wrap file's `## handoff` section, which `bun nv session --wrap` writes into the live
goal's handoff record, `data/goals/<slug>.handoff.json`, and `bun nv orient` prints back. Every future
session reads it in full, so it is a **bounded state file, not a changelog**.

- **Overwrite in place**, and describe where the work stands now, never the path taken.
- **State only.** A fact that outlives ten sessions goes elsewhere: a trap in the playbook, a decision
  in the rule's fragment under `docs/rules/` with a decision record for its reasoning, how something
  works in the crate's module doc.
- **Point, don't restate** — name the `rule:` token, plan paragraph or module doc that owns a fact.
- **Aim for about 60 lines.** Nothing measures it; never spend an iteration trimming it.
- In this order:
  1. `## State` — 3–8 lines: which milestone, what is on disk, what is blocked.
  2. `## Next group` — two to four related slices as a checklist, in order, under one line naming the
     **goal stage they are in** and the **file set they share** (same files, not same topic) —
     `**Stage 5: diagnostics, phase-gated** — one file set: …`. Each names the rule that specifies it
     (`rule:<topic>/<rule>`, or a record section when only the reasoning has it) and the anchors it
     touches, **repo-rooted in the item** — `crates/nvs-runtime/src/ctx/isolate.rs:116`, never a bare
     file name and never up in `## State` — because that is the only place `bun nv orient` reads them from
     to inline the code into the next pack, and `bun nv session --wrap` refuses an open item without one.
     The stage number is read the same way, to pick that stage's `context` overlay in the goal's record
     ([loop-authoring.md](loop-authoring.md) § 2); a group that names none gets the goal's base
     manifest, which is the wider pack and never a broken one.
  3. `## Backlog` — up to 6 one-line items, each with its owning doc; trim the stale ones. The next
     goal starts from its own handoff record, so what must survive a goal switch is a gap record
     under `data/gaps/`, naming the module that owes it and the goal that owns it.

**If the pack did not print something you needed, say so in the handoff**, naming the `context` field
that was missing it. That manifest is maintained by the sessions that discover its gaps.
