# Novis autonomous work session

You are one session of an unattended loop. A driver starts a fresh session after you exit, so
**everything the next session needs must be on disk before you finish.** Nothing in your context survives.

**One line already in your context is wrong for this repository.** The driver launches you under
`--permission-mode bypassPermissions`, and under that mode the harness tells you to make file changes
with `sed`, heredocs or short scripts rather than Read, Edit and Write. `AGENTS.md` rule 1 says the
opposite and **rule 1 wins**: Write and Edit for files, `peek.py` for reading, never file content through
a shell. The harness line saves permission prompts, and this mode has none to save.

## The six steps

`AGENTS.md` § *Session workflow* is authoritative for steps 1–5; this file adds only what the loop
changes about them, and step 6.

1. **Your orientation is already in this message — do not fetch it.** The driver piped
   `python tools/orient.py` in ahead of this prompt, narrowed to the goal's `[context]` manifest in
   `docs/agent/loop-goal.toml`. Re-running it spends three calls and about 20k of context on a pack you
   already hold. If it is genuinely absent, run it once and say so in the handoff — that is a driver bug.
   `python tools/brief.py` is the unscoped version; reach for it only for something outside the goal —
   then **edit the `[context]` field in `docs/agent/loop-goal.toml` that was missing its selector**. It
   is reloaded every session and widening it breaks nothing, so a gap you only describe in the handoff
   is one the next session pays for again. `modules` is the exception you may skip: the driver sweeps it
   from the paths your own commits touched.
2. **Take your item, then keep taking slices from the group while both hold:** the next one touches
   files you have already loaded, and you are under 120k of context with the previous one committed.
   Past 120k, stop and say in the handoff where you stopped. The gate and its measurement are
   `AGENTS.md` § *Session workflow* step 2.
3. **Verify once, at the end of the group, and start it before you write the wrap.** `python
   tools/verify.py --start` returns at once, `--wait` collects the verdict, and the wrap file gets
   written in between. A mid-work check is `--fast` or `-p <crate>`, never the full gate. Add a
   `valgrind` run for a new refcount edge (`docs/agent/commands.md`).
4. **Write the docs and the handoff once for the whole group**, and **choose the next group** — you hold
   the context that makes that cheap. A *trap* — something that looked like it should work and did not —
   earns one bullet in `docs/agent/playbook.md`, in the shape `docs/agent/conventions.md` § *A playbook
   bullet* gives, ending with the `[until: ...]` trailer `tools/playbook.py`'s module doc defines.
5. **Commit one slice at a time**, staging each slice's own files.
6. **Write one line to `.loop/status.txt`** (overwrite), then exit:
   - `CONTINUE <what you landed>` — the normal case.
   - `DONE <what goal was reached>` — the goal in `docs/agent/loop-goal.md` is met. Run `python
     tools/verify.py --doc` first and fix every broken doc link it names, then `python
     tools/owners.py --closes <slug>` and `python tools/playbook.py --closes <slug>` and close or
     re-owner every gap they name: those are the gates a goal meets only at its end, and the driver
     does not reach the goal while one is red. A tag is not a build.
   - `BLOCKED <the decision only the user can make>` — a tradeoff expensive to reverse. Prefer the safe
     option and a note in the handoff; the driver **holds** the run on this, waiting for the person who
     can answer it, and carries on from the tree as it stands when they lift the hold.

**Steps 4 to 6 are one wrap file and two calls.** The pack ends with the skeleton `python
tools/session.py --template` would print for this tree; fill it in and apply it with `python
tools/session.py --wrap <file>`. It validates every section before writing a byte, writes the docs,
and commits them with the slices — no second call, no `git add` by hand. **Then stop**: no second
verification, no re-reading the orientation, no trimming a doc to a length. The driver runs the
acceptance check itself, and stops after `--max-stalls` sessions without a commit.

## Context is the budget, and reading is where it goes

A session must finish under **200k**, and that is a quality ceiling: an agent starts missing what it
has read well before its window is full. `orient.py` starts you under 20k of it.

- **Read a big file in the region you need.** Whole file under about 400 lines; past that, locate the
  anchor and read around it.
- **Do not re-read what the pack printed** — the handoff, the rules, the record sections, the shapes,
  the traps. A rule's chapter body is the rule and a decision record is frozen reasoning; for another
  record section, slice it and name it in the handoff for `[context] adrs`.
- **Read with `peek.py`, not one probe at a time.** `python tools/peek.py A.rs:120-160 B.rs:@symbol
  rule:types/conversion C.md:"## 4" "crates/**/*.rs:re:pat"` takes as many targets as you have
  questions, and `--locate <symbol> ...` returns `file:line` anchors alone. A `rule:` citation is a
  target: paste the token and get the fragment, which is the rule.
- **Delegate a read-heavy search to a subagent, and keep its answer rather than its reading.** Its
  reads are charged to its own window. Send one for "where is X, and what are its anchors" over files
  you will not otherwise open. One measured run: 1 session in 39 delegated anything, while discovery
  and source reads took 53% of everything fetched — `python tools/loop-stats.py --attribute` is that
  number now.
- **What is safe to delegate, and what is not.** Safe: *where is X*, *how many of Y are there*, *what
  spelling does the corpus use*, over a tree you are not editing — and sent as the read-only agent
  type, `Explore`, which has no Edit or Write tool, so the boundary holds by construction rather than
  by instruction. Not safe, ever: writing anything; reading a file you are about to edit; deciding a
  design question; judging whether a check passed; the wrap. The handoff and the code must rest on
  what **you** read — a subagent's summary is a pointer to verify, not evidence to commit. The one
  carve-out in this repository is `dossier.py --partition`, which is a different protocol and says so.
- **When you have two or three independent searches, send them in one message.** They then run
  concurrently and cost you one round trip instead of three, which is the wall-clock half of the win;
  the context half you get either way. Independent means neither one's prompt depends on the other's
  answer — otherwise they are sequential and sending them together just guesses.
- **A delegated search gets the question, the place to look and the answer's shape — never the
  project.** It starts with its own copy of `AGENTS.md`. Ask for `file.rs:NN` anchors and one line
  each, never excerpts. The whole prompt:

      Search only under crates/nvs-stdlib/src. Find every CoreMethod row whose return type is
      CoreTy::Instance. Return one line each: `file.rs:NN  Class::member  -> instance name`.
      No excerpts, no commentary. If you find none, say so.
- **`python tools/verify.py` once, for the whole group** — and `--start` it *before* you write the
  wrap, so the build runs while you write. That is the one piece of parallelism that is free every
  single session, and step 3 above is where it belongs.

`python tools/loop-stats.py` measures all of this from `.loop/logs/`, and `--attribute` says which reads
put a session where it landed.

## The handoff's shape — `docs/agent/handoff.md`

Every future session reads this file in full, so it is a **bounded state file, not a changelog**.

- **Overwrite in place**, and describe where the work stands now, never the path taken.
- **State only.** A fact that outlives ten sessions goes elsewhere: a trap in `playbook.md`, a decision
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
     file name and never up in `## State` — because that is the only place `orient.py` reads them from
     to inline the code into the next pack, and `session.py --wrap` refuses an open item without one.
     The stage number is read the same way, to pick the goal's `[context.stage.N]` overlay
     ([loop-authoring.md](loop-authoring.md) § 2); a group that names none gets the goal's base
     manifest, which is the wider pack and never a broken one.
  3. `## Backlog` — up to 6 one-line items, each with its owning doc; trim the stale ones. A goal switch
     overwrites the whole handoff, so what must survive one goes in [carried-gaps.md](carried-gaps.md).

**If the pack did not print something you needed, say so in the handoff**, naming the `[context]` field
that was missing it. That manifest is maintained by the sessions that discover its gaps.
