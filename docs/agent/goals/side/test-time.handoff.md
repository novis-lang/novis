# Handoff

## State

**Side goal `test-time` has just started, and nothing of it has landed yet.** The goal: a check
runs again only when something it reads has changed. It runs in its own worktree under
`loop.py --side test-time`, over main's carried floor, and lands on `main` when it is green.

The design is the user's, decided on 2026-09-24 question by question. The goal file's
§ *Standing decisions* answers every call a session meets, and no session re-decides one.
§ *What is on disk today, measured* is the starting point: every cause there has a `file:line`.
Re-check a line number before you edit at it, because the chain run keeps committing to `main`
and each rebase brings those commits in.

**Never prove a cut with a sweep.** `impact.py --probe`, `loop.py --why` and single commands are
the tools. `--settle`, `--goal-only --full` and a full `verify.py` started to show that a cut works
are the mistake this goal exists to stop.

## Next group

**Stage 2: the bounded-units test, and the driver takes verify's record for every test shape.**
One file set: `crates/nvs-cli/src/script.rs`, `tools/loop.py`.

- [ ] **`EDITS` becomes 200** (`crates/nvs-cli/src/script.rs:2513`), and the test is renamed
      `units_held_stay_bounded_after_two_hundred_edits`. Its comment is rewritten whole.
- [ ] **`plain_crate_test` also recognises `--bin B [filter]` and `--lib [filter]`**
      (`tools/loop.py:2190`), and `crate_tests` and `verify_green` answer those checks from
      verify's record.
- [ ] **`loop.py --why "<check name>"`** prints `source: ...` and the key's inputs.

## Backlog

- **Stage 3: the probe table**, `tools/data/impact-probes.toml` plus `impact.py --probe`. Write
  it before any key moves: its red cells are the worklist for Stages 4 and 5.
- **Stage 4: one key for what the binary is built from.** This also fixes dossier's stale-binary
  bug.
- **Stage 5: the card tier.**
- **Stage 6: dossier in one pass.**
- **Stage 7: the proof profile.**
- **Stage 8: the smaller cuts.**
- When the last check goes green, the side run lands itself on `main`, and the first sweep after
  that re-keys once.
