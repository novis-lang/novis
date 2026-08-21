# Agent coordinator loop

Use this prompt to start (or restart) an autonomous coordinator session for MWL. The coordinator does not
write code itself — it drives a sequence of one-at-a-time subagent sessions, each of which does one
ordinary work session exactly as a human-driven session would (per `CLAUDE.md`), then reports back.

## The loop

Ask the user how long the loop should run. Make sure the target goal can be reached without interaction by the user. If the road to the target goal have any decisions that need be made along the way, ask beforehand, so the loops can work autonomously. Warn the user if the loops goal is to far ahead. Give the user a good entry point of what goal is optimal for this kind of loop.


1. Read `NEXT_SESSION_PROMPT.md` in the repo root.
2. Launch exactly **one** subagent (Agent tool, `subagent_type: general-purpose`, `run_in_background:
   false`). Never run two of these concurrently, and never launch the next one until this one has
   returned — they share one git working tree and one `NEXT_SESSION_PROMPT.md`, so concurrent writes would
   race. Give it a fully self-contained prompt: the complete text of `NEXT_SESSION_PROMPT.md`, plus these
   standing instructions:
   - Read `CLAUDE.md` first and follow it exactly, including the priority ordering and the "Ground rules
     enforced elsewhere" table.
   - Do the work described in the prompt above. If it lists several independent items, pick what fits in
     one focused session — the same judgment call a human session would make.
   - Run the relevant verification for what you touched (at minimum `cargo build`, `cargo test`, `cargo
     clippy --all-targets -- -D warnings`, `cargo fmt --check`).
   - Commit the work in small, focused commits, per `CLAUDE.md`'s "Keep work small, commit your work".
   - Before finishing, overwrite `NEXT_SESSION_PROMPT.md` with the next session's prompt, exactly as
     `CLAUDE.md`'s existing workflow instructions require.
   - If the entire plan (or the milestone you were told to focus on) is complete, say so explicitly instead
     of inventing busywork — in both the final report and `NEXT_SESSION_PROMPT.md`.
   - If you hit a decision only the user can make (a real tradeoff in performance/memory/usability/
     simplicity per `CLAUDE.md`'s last bullet), stop and say so instead of guessing.
   - End the final report with a short (5-10 line) summary only: what changed, what was verified, and the
     one-line status of `NEXT_SESSION_PROMPT.md`'s next step. No full diffs or file contents — the
     coordinator only needs the summary.
3. When the subagent returns, read its summary and the (now updated) `NEXT_SESSION_PROMPT.md`. Post one
   short status line to the user, then go back to step 1.
4. Stop when: the subagent reports the plan/milestone is complete, the subagent reports it's blocked on a
   decision only the user can make, or the user interrupts. Don't stop merely because many iterations have
   run.

## Constraints

- Never run more than one subagent at a time.
- Keep the coordinator's own context small: don't re-read full diffs, don't re-derive the plan yourself,
  don't second-guess a subagent's completed and verified work. If a summary looks incomplete or
  contradictory, spawn one more subagent to investigate rather than digging through the repo in the
  coordinator's own context.
- If this conversation's context gets auto-summarized mid-loop, the loop is still resumable without loss:
  the persistent state lives in the repo (`NEXT_SESSION_PROMPT.md`, git history,
  `docs/implementation-plan.md`), not in the coordinator's memory. Just re-read `NEXT_SESSION_PROMPT.md`
  and continue from step 1.
