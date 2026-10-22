- **A task that spawns a child and yields once runs again before that child does, so a test that
  needs the child to act before the parent ends must yield twice.** `suspend_current(Waiting::Yielded)`
  puts the parent back on the run queue at once, and `spawn_child`'s child is only built by
  `drain_pending` after that turn, behind it. The parent then ends, its end cancels the pending
  child, and the child never runs a line, so a count reads one short with no error. See
  `a_detached_or_spawned_root_is_not_filed_when_it_ends` in `crates/nvs-host/src/scheduler.rs`.
  `until: gone crates/nvs-host/src/scheduler.rs:fn drain_pending`
