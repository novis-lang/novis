# MWL autonomous work session

You are one session of an unattended loop. A driver script restarts a fresh session after you exit, so
**everything the next session needs must be written to disk before you finish.** Nothing in your context
survives.

## Do

1. Run `python .claude/brief.py`. Read `CLAUDE.md` and follow it exactly.
2. Read `NEXT_SESSION_PROMPT.md`. Do the work it names — if it lists several independent items, pick the
   one that fits a single focused session, the same judgment a human session would make.
3. Verify what you touched: at minimum `cargo build`, `cargo test`,
   `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
4. Commit in small focused commits, per `CLAUDE.md`'s "Keep work small, commit your work".
5. Overwrite `NEXT_SESSION_PROMPT.md` under the handoff contract below.
6. Write exactly one line to `.claude/loop-status.txt` (overwrite, no newline needed):
   - `CONTINUE <one-line summary of what you landed>` — normal case.
   - `DONE <what goal was reached>` — the goal in `.claude/loop-goal.md` is met.
   - `BLOCKED <the decision only the user can make>` — a real tradeoff per `CLAUDE.md`'s last bullet.
     Use this sparingly: prefer the safe option and note it in the handoff. Reserve `BLOCKED` for a
     decision that would be expensive to reverse.

The driver stops the loop on `DONE` or `BLOCKED`, and on three consecutive sessions that produce no commit.

## The goal

`.claude/loop-goal.md` holds the loop's target. Steer toward it. Do not invent busywork once it is reached
— write `DONE` instead.

## Handoff contract for `NEXT_SESSION_PROMPT.md`

This file is read in full by every future session, so it is a **bounded state file, not a changelog**.
Hard rules:

- **80 lines maximum.** If you are over, you are carrying history that belongs in `git log`.
- **Overwrite in place.** Never append a "landed this session" section on top of the previous one. Describe
  where the work *stands now*, not the path taken to get here.
- **Point, don't restate.** Name the ADR / the plan paragraph / the crate's module doc comment that owns a
  fact. One sentence of pointer beats a paragraph of summary — a copy here goes stale silently.
- Required shape, in this order:
  1. `## State` — 3-8 lines: which milestone, what is on disk, what is blocked.
  2. `## Next` — the single most valuable next slice, with the ADR/plan section that specifies it.
  3. `## Backlog` — up to 6 one-line items, each with its owning doc. Trim the ones that went stale.
- Detail about *how* something was implemented belongs in that crate's module doc comment or the plan's
  milestone paragraph — the homes `CLAUDE.md` already names — never here.
