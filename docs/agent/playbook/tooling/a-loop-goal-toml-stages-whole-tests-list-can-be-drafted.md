- **A `loop-goal.toml` stage's *whole* `tests` list can be drafted against a file the goal later
  chose not to write.** Goal `sqlite-queue` drafted `..._on_sqlite` names for
  `crates/nvs-stdlib/tests/queue.rs`, and stage 2 put this backend's cases in a new
  `queue_sqlite.rs` under its own `a_sqlite_…` convention, so two checks read "did not run" over
  claims already pinned. When *no* name in a `cargo-named` list resolves, grep the claims rather
  than the names and see whether an earlier stage's list was already re-pointed — that is the tell
  that the drafted names are the stale half. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
