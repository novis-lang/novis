# Novis loop repair session

You are one session of the unattended loop, started for one job. The driver stopped on the verdict
at the end of this prompt, and you fix what caused it, so the run carries on without waiting for a
person.

The orientation pack follows the verdict. It is context, not a worklist: do not take the goal's next
item. `AGENTS.md` still holds in full, rule 1 included: the driver runs you under
`--permission-mode auto`, so a call outside `.claude/settings.json`'s allowlist is reviewed and may be
denied, and files are still written with Write and Edit, never through a shell.

## The five steps

1. **Find the cause.** Start with the verdict, then the evidence behind it. `.loop/log.md` is the
   ledger: read every `goal check:` line of this goal, not only the last. The session log it names,
   `.loop/logs/<run>-NNNN.log`, has each check's whole output in its `loop_output` records. `bun nv
   loop --goal-only` runs the acceptance list again with the code on disk now.
   [coordinator.md](coordinator.md) says what each verdict means and when the driver gives one.
2. **Fix the cause, not the symptom.** The loop's own tooling under `tools/` is in scope, and so is a
   check that is itself wrong. A test renamed to match, a check removed or a bound loosened until it
   passes is not a fix. If the verdict came from something already fixed on disk, nothing needs
   changing: say so in step 5.
3. **Verify once:** `bun nv verify`, plus whatever the fix itself needs.
4. **Commit each fix on its own.** Write the message to a file under `.agent-tmp/` and commit with
   `git commit -F`. The shape is [conventions.md](conventions.md) § *A commit message*, and the
   `commit-msg` hook refuses trailer lines. Leave the goal's handoff record alone unless the fix
   changes what the next session should do; a `## handoff` section in a `bun nv session --wrap` file
   is what rewrites it. A trap that cost you time gets a playbook bullet, as in any
   session.
5. **Write one line to `.loop/status.txt`** (overwrite), then exit:
   - `CONTINUE <what you fixed>`: the normal case. The driver runs its sweep, and the goal's sessions
     carry on.
   - `DONE <what goal was reached>`: the verdict was a refused DONE claim, and your fix makes the
     goal's acceptance list pass. The gates are the ones [session-prompt.md](session-prompt.md) step 6
     names for `DONE`.
   - `BLOCKED <the decision only the user can make>`: the fix needs a tradeoff that is expensive to
     reverse, a change to the language, or a rule that decides against it. Do not guess. The driver
     holds the run for the user.

Context is capped as in any session: `AGENTS.md` § *Session workflow* step 2.

## The verdict the run stopped on
