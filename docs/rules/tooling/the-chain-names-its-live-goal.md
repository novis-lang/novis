The loop's live goal is `data/chain.json`'s `live`, a slug on its `goals` list, committed like every other
record. So a fresh clone, CI and a git hook read the same live goal the driver works on, and nothing
about which goal is live is kept only on the machine that ran the loop. `bun nv chain` edits the list
and never `live`. The driver's goal switch moves it: once a goal's sweep, its Linux legs and its
goal-end gates are green, the next goal on the list becomes `live` in a commit of that one file.

A goal's floor is a view, never a copy. The plan the driver runs for a goal is its record with every
check of every goal in front of it on the list carried in under one stage titled `floor`, each check
once. A goal that has walked keeps its checks for this reason, and a switch copies or retires nothing.
A walked test check that the permanent suite already runs graduates: a plain `cargo test -p <crate>` or
an `nvs test` over the conformance or differential tree is carried as its crate's or its tree's whole
run, once, and `bun nv chain --check` fails when a test or case such a check names is gone.
The runtime state under `.loop/` — the memo, the run's counts, the gate verdicts — stays git-ignored,
because it is per machine and a fresh clone owes nothing it holds.
