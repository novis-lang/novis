- **A `loop-goal.toml` check can name a test the tree already declares under a *different* name.**
  Stage 5 asked for `every_migration_member_is_registered`, which no crate declares, while
  `spec_registry_coverage.rs` has carried `every_migration_member_row_names_a_registered_member` —
  the same walk over the same fixture — throughout, so the check read "did not run" against a green
  tree. `grep` a check's test name against `crates/` first: a near miss on a real name is a
  shorthand to fix in the goal file, not a test to write. [until: reviewed 2026-09-09]
