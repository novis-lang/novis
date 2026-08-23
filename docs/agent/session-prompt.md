# MWL autonomous work session

You are one session of an unattended loop. A driver script starts a fresh session after you exit, so
**everything the next session needs must be written to disk before you finish.** Nothing in your context
survives.

## The five steps

Run these in order, then **stop**. This is `AGENTS.md` § *Session workflow*, with the loop's own step 6
added; that file is authoritative for steps 1–5.

1. **Orient.** `python tools/brief.py`, then `AGENTS.md`, then `docs/agent/handoff.md`. Do the work the
   handoff names — if it lists several independent items, pick the one that fits a single focused session,
   the same judgment a human session would make. `docs/agent/loop-goal.md` holds the loop's target; steer
   toward it and do not invent busywork once it is reached.
2. **Do the work.** One focused slice.
3. **Verify what you touched:** at minimum `cargo build`, `cargo test`,
   `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, plus a `valgrind` run for any new
   refcount edge (`AGENTS.md` § *Commands*). **This is the only place verification happens.**
4. **Write the docs and the handoff.** The plan's status block, any doc the change invalidates, then
   overwrite `docs/agent/handoff.md` under the contract below.
5. **Commit.** Small focused commits, per `AGENTS.md`.
6. **Write one line to `.loop/status.txt`** (overwrite, no newline needed), then exit:
   - `CONTINUE <one-line summary of what you landed>` — normal case.
   - `DONE <what goal was reached>` — the goal in `docs/agent/loop-goal.md` is met.
   - `BLOCKED <the decision only the user can make>` — a real tradeoff per `AGENTS.md`'s last section.
     Use this sparingly: prefer the safe option and note it in the handoff. Reserve `BLOCKED` for a
     decision that would be expensive to reverse.

**After step 6 the session is over.** Do not re-run `cargo build`/`test`/`clippy`/`fmt`, do not re-read the
digest, do not re-check any doc against a length. Prose cannot break a build, so a second verification pass
can only find what the first one already reported — it burns the session's remaining budget for nothing.
The driver runs the acceptance check itself, every iteration; that is the machine's job, not yours.

The driver stops the loop on `DONE` or `BLOCKED`, and after `--max-stalls` consecutive sessions with no
commit.

## Handoff contract for `docs/agent/handoff.md`

This file is read in full by every future session, so it is a **bounded state file, not a changelog**.

- **Overwrite in place.** Never append a "landed this session" section on top of the previous one. Describe
  where the work *stands now*, not the path taken to get here.
- **Point, don't restate.** Name the ADR / the plan paragraph / the crate's module doc comment that owns a
  fact. One sentence of pointer beats a paragraph of summary — a copy here goes stale silently.
- **Aim for about 80 lines.** If you are well over, you are carrying history that belongs in `git log`.
  This is a target for the author, not a check: nothing measures it, and a handoff that lands at 90 lines
  is finished. Never spend an iteration trimming it.
- Required shape, in this order:
  1. `## State` — 3-8 lines: which milestone, what is on disk, what is blocked.
  2. `## Next` — the single most valuable next slice, with the ADR/plan section that specifies it.
  3. `## Backlog` — up to 6 one-line items, each with its owning doc. Trim the ones that went stale.
- Detail about *how* something was implemented belongs in that crate's module doc comment or the plan's
  milestone paragraph — the homes `AGENTS.md` already names — never here.
