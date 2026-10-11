The loop's live goal is the first slug of the `chain:` list in the front matter of `docs/goals/README.md`,
committed like every other file. So a fresh clone, CI and a git hook read the same live goal looper works
on, and nothing about which goal is live is kept only on the machine that ran the loop. `looper goal new`
and `looper goal move` edit the list.

A reached goal's file under `docs/goals/` is deleted, its slug leaves the chain, and the next slug is the
live goal. Nothing is archived, and `git log` is the history. A decision that only a goal's prose holds is
moved to its home before the goal is reached. The frozen records under `docs/decisions/` keep the goals
they name. A reached goal kept on disk would hold nothing a later session reads, and would still be a
document that cites code and drifts from it, with every tool carrying a branch to tell it from a live
one. A regression in finished work is caught by the suite that tests it, or by nothing.

No goal's checks are carried as a floor: `looper.toml` sets `carry_to_floor = false`. A goal's plan is its
own front matter and nothing else. The permanent suites and `bun nv verify` protect finished work, and a
check that proves more than a suite becomes a test before its goal is reached.

The runtime state stays git-ignored: the selection store in `.cache/select.sqlite`, and looper's run
state under `.loop/`. It is per machine, and a fresh clone owes nothing it holds.

**Declined:**
- **Keeping the live goal in the git-ignored run state.** A fresh clone has no such file, so it has no
  live goal: the orientation and CI read nothing, and a push hook that needs the live goal refuses
  every push.
- **Carrying a reached goal's test checks into a floor, whole or reduced to one check per crate and
  per case tree.** Most of them re-run what `bun nv verify` already runs, and the floor grows with
  every goal reached. A check written for an early goal also pins that goal's view of the engine, so
  when the engine moves on a session has to edit the record of a goal it has never seen.
- **Keeping the floor and deleting only a reached goal's prose.** Its checks would still have to stay
  current against code that moves on, which is the drift deleting the goal removes.
- **Archiving reached goals under another directory.** An archive is a document nobody reads that
  still matches every search, and `git log` already holds it.
