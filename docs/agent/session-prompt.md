# MWL autonomous work session

You are one session of an unattended loop. A driver script starts a fresh session after you exit, so
**everything the next session needs must be written to disk before you finish.** Nothing in your context
survives.

## The five steps

Run these in order, then **stop**. This is `AGENTS.md` § *Session workflow*, with the loop's own step 6
added; that file is authoritative for steps 1–5.

1. **Orient in one call**, then read `AGENTS.md`:

       python tools/brief.py && cat docs/agent/handoff.md docs/agent/playbook.md docs/agent/conventions.md

   That is the map, where the work stands, the traps, and the shape of anything you are about to write —
   one turn, not four. `docs/agent/loop-goal.md` holds the loop's target; steer toward it and do not
   invent busywork once it is reached.
2. **Do the work — as much of the group as fits under the context ceiling.** `## Next group` lists related
   items and the file set they share. **Take the first. Take a second only if it touches files you have
   already loaded and the first left you well short of the ceiling. Never take a third.** Anything you do
   not reach stays ticked-off-able in the group for the next session; say in the handoff where you stopped.
3. **Verify once, at the end of the group:** `python tools/verify.py`, plus a `valgrind` run for any new
   refcount edge (`AGENTS.md` § *Commands*). **This is the only place verification happens**, and the
   whole group shares one run — it is the same build either way.
4. **Write the docs and the handoff, once for the whole group.** The plan's status block, any doc the
   change invalidates, then overwrite `docs/agent/handoff.md` under the contract below. If the session
   cost you a *trap* — something that looked like it should work and did not — add one bullet to
   `docs/agent/playbook.md` instead. **Choosing the next group is part of this step**, not the next
   session's problem: you are holding the context that makes it cheap.
5. **Commit — one per slice**, staging each slice's own files so `git log` still reads a slice at a time.
6. **Write one line to `.loop/status.txt`** (overwrite, no newline needed), then exit:
   - `CONTINUE <one-line summary of what you landed>` — normal case.
   - `DONE <what goal was reached>` — the goal in `docs/agent/loop-goal.md` is met.
   - `BLOCKED <the decision only the user can make>` — a real tradeoff per `AGENTS.md`'s last section.
     Use this sparingly: prefer the safe option and note it in the handoff. Reserve `BLOCKED` for a
     decision that would be expensive to reverse.

**After step 6 the session is over.** Do not re-run the verification, do not re-read the digest, do not
re-check any doc against a length. Prose cannot break a build, so a second verification pass can only find
what the first one already reported — it burns the session's remaining budget for nothing. The driver runs
the acceptance check itself, every iteration; that is the machine's job, not yours.

The driver stops the loop on `DONE` or `BLOCKED`, and after `--max-stalls` consecutive sessions with no
commit.

## Your clock is your turn count

A session's wall time is very nearly **the number of tool calls it makes, times a constant** — about 7.6 s
a call, three quarters of it spent waiting on the model rather than on `cargo`. Those calls split roughly
**a quarter orientation before the first edit, half actual work, a quarter verify-plus-docs-plus-commit**:
**over half of every session is fixed cost**, paid once no matter how little work sits between the two
halves of it. That is the whole reason step 2 takes a *group*.

**But context, not the clock, is what caps a session.** The window is 1M and nothing compacts — and that
is not the limit that matters, because an agent starts missing what it has already read long before its
window is full. **The ceiling is a fixed 200k**, a session starts at about 42k of it, and the measured
sessions have been finishing *over* the line doing one slice each. So the saving on offer here is reading
less, not doing more, and the two bullets below that say so outrank the rest of this list.

`python tools/loop-stats.py` re-derives every number in this section from `.loop/logs/`, so none of them
has to be believed. Run it rather than trusting the sentence above; it is also the first step of setting a
new goal ([loop-authoring.md](loop-authoring.md) § *Measure first*).

So:

- **Batch every independent probe into one message.** Several tool calls in a single message run
  concurrently and each keeps its own exit status. Four greps to locate a symbol is one turn, not four.
  Serialize only what genuinely depends on a previous answer. (`AGENTS.md` § *Commands* has the rule and
  the one thing it does not weaken.) **Measured sessions do this at a rate of zero** — 297 consecutive
  tool calls, not one of them sharing a message with another. Batching is the single largest saving on
  this list and it is the one nobody collects.
- **`python tools/brief.py`'s map already answers "which file is this in".** One line per module and the
  file:line of the definitions sessions grep for most. It also prints the next free diagnostic code and
  ADR number, so neither needs deriving.
- **`docs/agent/conventions.md` already answers "what shape does this take".** Do not open an existing
  `.mwlt` case, registry row, ADR or commit to copy its form — that answer is identical every session and
  it is written down.
- **Edit the plan's status block with `python tools/plan.py --set`,** never by locating its bytes and
  splicing them. That cycle averaged 11.6 tool calls a session.
- **Read a small file once; read a big one in the region you need.** Under about 400 lines, or when you
  will touch most of it, take the whole file — a sequence of narrow windows costs a turn each. Past that,
  `grep -n` for the anchor and read around it: one `cat` of an 1,100-line module spends a tenth of the
  whole session's context budget in a single call. This is the one place turn economy and context economy
  disagree, and context wins, because it degrades the work rather than merely slowing it.
- **Do not re-read what the orientation call already printed.** The handoff, the playbook, the conventions
  and the brief are in your context from call one; opening any of them again is pure cost.
- **Verify with one call, once for the whole group**, per step 3. A measured session ran `verify.py` four
  times for one slice; three of those runs rebuilt the same tree to learn the same thing.

## Handoff contract for `docs/agent/handoff.md`

This file is read in full by every future session, so it is a **bounded state file, not a changelog**.

- **Overwrite in place.** Never append a "landed this session" section on top of the previous one. Describe
  where the work *stands now*, not the path taken to get here.
- **State only.** A fact that will still be true in ten sessions does not belong here: a trap goes in
  `playbook.md`, a design decision in its ADR, an explanation of how something works in that crate's module
  doc comment. This file carried ~96 lines of permanent lore until it was split out, and every session
  regenerated and quietly reworded all of it.
- **Point, don't restate.** Name the ADR / the plan paragraph / the crate's module doc comment that owns a
  fact. One sentence of pointer beats a paragraph of summary — a copy here goes stale silently.
- **Aim for about 60 lines.** If you are well over, you are carrying history that belongs in `git log`.
  This is a target for the author, not a check: nothing measures it, and a handoff that lands at 70 lines
  is finished. Never spend an iteration trimming it.
- Required shape, in this order:
  1. `## State` — 3-8 lines: which milestone, what is on disk, what is blocked.
  2. `## Next group` — **two to four related slices as a checklist**, in the order to do them, each with
     the ADR/plan section that specifies it, under one line naming **the file set they share**. Related
     means *same files*, not same topic: that shared set is what makes slices 2 and 3 cost a third of
     slice 1. A checklist, because a session that lands two of three leaves the rest tickable rather than
     re-derived — the next session strikes what is done and carries on. Group nothing that would send a
     session re-orienting; leave that item in `## Backlog` for a group of its own.
  3. `## Backlog` — up to 6 one-line items, each with its owning doc. Trim the ones that went stale.
