# Novis autonomous work session

You are one session of an unattended loop. A driver script starts a fresh session after you exit, so
**everything the next session needs must be written to disk before you finish.** Nothing in your context
survives.

**One instruction already in your context is wrong for this repository, and it is not this file's.** The
driver launches you under `--permission-mode bypassPermissions`, and under that mode the harness adds a
standing line telling you to make file changes "with `sed`, heredocs, or short scripts, rather than using
the dedicated Read, Edit, or Write tools". `AGENTS.md` rule 1 says the opposite, and **rule 1 wins**:
create and edit files with Write and Edit, read with `peek.py`, and never carry file content through a
shell — the shell parses apostrophes and backticks before it runs anything, which is how a doc comment
gets mangled. The harness line is generic advice for saving permission prompts, and there are none to save
here: the mode already grants everything. `python tools/loop-stats.py` counts the breaches; before this
paragraph existed it found them in 80 of 101 sessions, 333 calls, which is the whole reason it is here.

## The five steps

Run these in order, then **stop**. This is `AGENTS.md` § *Session workflow*, with the loop's own step 6
added; that file is authoritative for steps 1–5.

1. **Your orientation is already in this message — do not fetch it.** The driver ran
   `python tools/orient.py` and piped its output in ahead of this prompt: where the work stands, your item
   in full, the goal's standing decisions, the rules and ADR sections it lives inside, the map lines for
   its files, the shapes you are about to write, and the traps that apply to them. **Running the script
   yourself costs three calls and about 20k of context for a pack you already hold** — the harness spills a
   result that size to a file, and reading it back is the expensive half. If it is genuinely not there, run
   `python tools/orient.py` once and say so in the handoff, because that is a driver bug.

   The pack is **narrowed on purpose** — the goal's `[context]` manifest in `docs/agent/loop-goal.toml`
   selects it, and everything it leaves out is context you are not charged for. `python tools/brief.py` is
   the unscoped version; reach for it only when you genuinely need something outside the goal, and say so
   in the handoff so the manifest gains the selector.
2. **Do the work — as much of the group as fits under the context ceiling.** `orient.py` prints your item
   in full and the rest of the group one line each. **Take the first. Then keep taking slices while both
   are true: the next touches files you have already loaded, and you are under 120k of context with the
   previous one committed.** The context test is the one that matters and it is a *measurement*, not
   a judgement — the first session run under a two-slice rule with no number on it took § 9's two
   collections and ended at 235,448 against a 200,000 ceiling. 120k leaves the ~45k a *hard* slice has
   historically cost plus the ~33k tail of verification, docs, handoff and commits — and a slice that only
   writes a test over landed work costs a fraction of that, which is the whole reason the gate counts
   context and not slices. If you are past it, stop and say in the handoff where you stopped; what you do
   not reach stays ticked-off-able for the next session, and the ceiling is a quality number, not a
   capacity one. The gate itself and the measurement behind it live in `AGENTS.md` § *Session workflow*
   step 2, which is their only home.
3. **Verify once, at the end of the group — and start it before you write the wrap.** `python
   tools/verify.py --start` returns at once, `python tools/verify.py --wait` collects it with its exit
   status, and the wrap file gets written in between: it is the same steps and the same verdict, with the
   42 seconds overlapping prose that cannot fail. Add a `valgrind` run for any new refcount edge
   (`docs/agent/commands.md`). **This is the only place verification happens**, and the whole group shares
   one run — it is the same build either way. A mid-work check is `--fast` or `-p <crate>`, never the full
   gate.
4. **Write the docs and the handoff, once for the whole group.** The plan's status block, any doc the
   change invalidates, and `docs/agent/handoff.md` overwritten under the contract below. If the session
   cost you a *trap* — something that looked like it should work and did not — add one bullet to
   `docs/agent/playbook.md`. **Choosing the next group is part of this step**, not the next session's
   problem: you are holding the context that makes it cheap.
5. **Commit — one per slice**, staging each slice's own files so `git log` still reads a slice at a time.
6. **Write one line to `.loop/status.txt`** (overwrite, no newline needed), then exit:
   - `CONTINUE <one-line summary of what you landed>` — normal case.
   - `DONE <what goal was reached>` — the goal in `docs/agent/loop-goal.md` is met.
   - `BLOCKED <the decision only the user can make>` — a real tradeoff. Use this sparingly: prefer the safe
     option and note it in the handoff. Reserve `BLOCKED` for a decision expensive to reverse.

**Steps 4 to 6 are two calls, not thirty.** `python tools/session.py --template` prints the wrap file's
format already carrying what the tail used to re-derive by hand — every count the tree has moved past as an
applicable `## plan-edit:`, the playbook's headings, and the `## commit:` for the docs the wrap writes. Fill
it in — `## playbook: <Heading>` for a trap, `## handoff` for the whole handoff, a `## commit: <paths>` per
slice, `## status` — and apply it with `python tools/session.py --wrap <file>`. It validates every section
before writing anything, so it either does the whole tail or does none of it and tells you why; a dry run
buys only the same refusal a call earlier. **One wrap writes the docs and commits them** — they are on disk
before any commit is staged, and anything it wrote that no `## commit:` names joins the last one, so there
is never a second call or a hand-rolled `git add` for the docs. Measured before the tool existed, this tail
was **33 of a session's 98 calls and 42% of its token bill**, because context is at its peak by the time you
reach it.

**After step 6 the session is over.** Do not re-run the verification, do not re-read the orientation, do
not re-check any doc against a length. Prose cannot break a build, so a second verification pass can only
find what the first one already reported. The driver runs the acceptance check itself, every iteration;
that is the machine's job, not yours.

The driver stops the loop on `DONE` or `BLOCKED`, and after `--max-stalls` consecutive sessions with no
commit.

## Context is the budget, and reading is where it goes

A session must finish under **200k**, and that is a *quality* ceiling, not a capacity one: an agent starts
missing what it has already read long before its window is full, and a session that degrades produces work
the acceptance test then rejects. `orient.py` is built to start you at well under 20k of it. What you do
with the rest is the whole game.

- **Read a big file in the region you need.** Whole file under about 400 lines, or when you will touch most
  of it; past that, `grep -n` for the anchor and read around it. One `cat` of an 1,100-line module spends a
  tenth of the session's budget in a single call.
- **Do not re-read what `orient.py` already printed.** It is in your context from call one.
- **Do not open a whole ADR.** `orient.py` printed the sections the goal named; if you need another, slice
  it — `sed -n` around the heading — and say in the handoff which section to add to `[context] adrs`.
- **Read with `peek.py`, not one `grep` at a time.** `python tools/peek.py A.rs:120-160 B.rs:@symbol
  C.md:"## 4" "crates/**/*.rs:re:pattern"` answers as many questions as you can ask in one call, and
  `--locate <symbol> ...` returns `file:line` anchors with no bodies at all. This replaces the *rule* that
  used to sit here, which asked sessions to batch independent probes into one message: measured over a
  full run they did that **0 times in 3,647 calls**, including runs of 52 and 57 consecutive greps. The
  saving was real and nobody ever collected it, so it is a tool now.
- **Delegate a read-heavy search to a subagent, and keep its findings rather than its reading.** A
  subagent has its own window: what it reads is charged to *that* window and only its answer comes back to
  yours. `python tools/loop-stats.py --attribute` charges **19% of a session to discovery** and reports
  **zero** subagents ever spawned across 39 sessions, so with batching now handled by `peek.py` this is
  the largest saving still going uncollected — and the one no tool can take for you. Send one when
  the question is "where is X, and what are its anchors" over files you will not otherwise open — a spec
  section's rows, every call site of a helper, which of forty cases already covers a member. Do **not**
  send one to write code, to decide anything, or to read a file you are about to edit: a slice's own files
  belong in your window, and the handoff you write must rest on what you actually read.
- **A delegated search gets the question and the constraints, not the project.** It starts with its own
  copy of `AGENTS.md`, so repeating any of that wastes both windows. The whole prompt is: the question,
  where to look, and *what to hand back* — `file.rs:NN` anchors and one line each, never pasted excerpts,
  because an excerpt in the answer charges your window for the reading you delegated. Worked shape:

      Search only under crates/nvs-stdlib/src. Find every CoreMethod row whose return type is
      CoreTy::Instance. Return one line each: `file.rs:NN  Class::member  -> instance name`.
      No excerpts, no commentary. If you find none, say so.
- **`python tools/verify.py` once, for the whole group.** A measured session ran it four times for one
  slice; three of those rebuilt the same tree to learn the same thing.

`python tools/loop-stats.py` re-derives every number above from `.loop/logs/`, and
`python tools/loop-stats.py --attribute` says which *reads* put a session where it landed. Run them rather
than believing this section.

## Handoff contract for `docs/agent/handoff.md`

This file is read in full by every future session, so it is a **bounded state file, not a changelog**.

- **Overwrite in place.** Never append a "landed this session" section on top of the previous one. Describe
  where the work *stands now*, not the path taken to get here.
- **State only.** A fact that will still be true in ten sessions does not belong here: a trap goes in
  `playbook.md`, a design decision in its ADR, an explanation of how something works in that crate's module
  doc comment.
- **Point, don't restate.** Name the ADR / the plan paragraph / the module doc that owns a fact. One
  sentence of pointer beats a paragraph of summary — a copy here goes stale silently.
- **Aim for about 60 lines.** A target for the author, not a check: nothing measures it, and a handoff that
  lands at 70 lines is finished. Never spend an iteration trimming it.
- Required shape, in this order:
  1. `## State` — 3-8 lines: which milestone, what is on disk, what is blocked.
  2. `## Next group` — **two to four related slices as a checklist**, in the order to do them, each with
     the ADR section that specifies it **and the `file.rs:NN` anchors it touches**, under one line naming
     **the file set they share**. Related means *same files*, not same topic. The anchors are not optional:
     you can resolve them now for nothing, and a session without them spends ten `grep`s re-deriving what
     you already knew. They go **in the item, repo-rooted** — `crates/nvs-runtime/src/ctx/isolate.rs:116`,
     never `isolate.rs:116` and never up in `## State` — because that is the only place and form `orient.py` reads
     them from to inline the code into the next pack, and `session.py --wrap` refuses an open item without
     one.
  3. `## Backlog` — up to 6 one-line items, each with its owning doc. Trim the ones that went stale.

**If `orient.py` did not print something you needed, say so in the handoff**, naming the `[context]` field
that was missing it. That manifest is the only thing standing between the next session and the whole
repository, and it is maintained by the sessions that discover its gaps.
