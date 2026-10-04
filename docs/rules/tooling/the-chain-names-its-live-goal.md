The loop's live goal is `data/chain.json`'s `live`, a slug on its `goals` list, committed like every other
record. So a fresh clone, CI and a git hook read the same live goal the driver works on, and nothing
about which goal is live is kept only on the machine that ran the loop. `bun nv chain` edits the list
and never `live`.

The driver's goal switch moves `live` and deletes the goal it leaves. Once a goal's sweep, its Linux
legs and its goal-end gates are green, the switch deletes the goal's prose, its record and its handoff
record, removes its slug from `goals`, makes the next goal `live`, renders the goal plan and commits all
of it as one commit. Nothing is archived, and `git log` is the history. A decision that only a goal's
prose holds is moved to its home before the goal is reached, and `bun nv chain --check` fails on prose
outside a goal's own files that names a goal not on the chain.

No goal's checks are carried as a floor. The live goal is first on the chain, so the plan the driver
runs for a goal is its own record and nothing else. The permanent suites and `bun nv verify` protect
finished work, and a check that proves more than a suite becomes a test before its goal is reached.
`goal-closeout` is the first goal this rule covers; the goals walked before it were deleted once, by that
goal, and never by the rule.

The runtime state stays git-ignored: the selection store in `.cache/select.sqlite`, and the run's
counts and the gate verdicts under `.loop/`. It is per machine, and a fresh clone owes nothing it holds.
