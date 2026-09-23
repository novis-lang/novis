- **A stage the handoff calls unwritten can be entirely on disk under other names, and a
  `cargo-named` check only ever reads names.** Goal `m5-proofs`'s stage 2 landed both deadlock tests
  and its `.nvst` case in one commit spelled `a_deliberate_deadlock_is_ended_by_the_deadline`, while
  the check names `two_tasks_waiting_on_each_others_channel_are_ended_by_the_groups_deadline`, so
  two sessions read "did not run" as work outstanding. When a check reports a test that did not run,
  `git log -S` the *claim* before writing it: where the assertion is already there under another
  name and no other check names that name, the whole fix is the rename.
  [until: reviewed 2026-09-14]
