# MWL autonomous work session

You are one session of an unattended loop. A driver script starts a fresh session after you exit, so
**everything the next session needs must be written to disk before you finish.** Nothing in your context
survives.

## The five steps

Run these in order, then **stop**. This is `AGENTS.md` § *Session workflow*, with the loop's own step 6
added; that file is authoritative for steps 1–5.

1. **Orient.** `python tools/brief.py`, then `AGENTS.md`, then `docs/agent/handoff.md` for where the work
   stands, `docs/agent/playbook.md` for the traps, and `docs/agent/conventions.md` for the shape of
   anything you are about to write. Do the work the handoff names — if it lists several
   independent items, pick the one that fits a single focused session, the same judgment a human session
   would make. `docs/agent/loop-goal.md` holds the loop's target; steer toward it and do not invent
   busywork once it is reached.
2. **Do the work.** One focused slice.
3. **Verify what you touched:** `python tools/verify.py`, plus a `valgrind` run for any new refcount edge
   (`AGENTS.md` § *Commands*). **This is the only place verification happens.**
4. **Write the docs and the handoff.** The plan's status block, any doc the change invalidates, then
   overwrite `docs/agent/handoff.md` under the contract below. If the session cost you a *trap* — something
   that looked like it should work and did not — add one bullet to `docs/agent/playbook.md` instead.
5. **Commit.** Small focused commits, per `AGENTS.md`.
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

A session's wall time is very nearly **the number of tool calls it makes, times a constant** — the work
itself is a rounding error beside it, and the loop's measured sessions spent 70–89% of their clock waiting
on the model rather than on `cargo`. Two thirds of that went to orientation: two fifths of every session's
tool calls were read-only probes asking where something lives, issued one at a time.

So:

- **Batch every independent probe into one message.** Several tool calls in a single message run
  concurrently and each keeps its own exit status. Four greps to locate a symbol is one turn, not four.
  Serialize only what genuinely depends on a previous answer. (`AGENTS.md` § *Commands* has the rule and
  the one thing it does not weaken.)
- **`python tools/brief.py`'s map already answers "which file is this in".** One line per module and the
  file:line of the definitions sessions grep for most. It also prints the next free diagnostic code and
  ADR number, so neither needs deriving.
- **`docs/agent/conventions.md` already answers "what shape does this take".** Do not open an existing
  `.mwlt` case, registry row, ADR or commit to copy its form — that answer is identical every session and
  it is written down.
- **Edit the plan's status block with `python tools/plan.py --set`,** never by locating its bytes and
  splicing them. That cycle averaged 11.6 tool calls a session.
- **Read a file once, not in slices.** A sequence of narrow `sed -n` windows costs a turn each and usually
  more tokens in total than the whole file would have.
- **Verify with one call**, per step 3.

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
  2. `## Next` — the single most valuable next slice, with the ADR/plan section that specifies it.
  3. `## Backlog` — up to 6 one-line items, each with its owning doc. Trim the ones that went stale.
