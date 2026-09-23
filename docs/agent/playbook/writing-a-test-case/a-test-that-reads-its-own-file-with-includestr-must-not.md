- **A test that reads its own file with `include_str!` must not spell its needle as a literal.** The
  scan finds the assertion's own source and answers about that instead of about the code.
  `affected_is_the_matched_count_and_changed_is_mysql_only` in `crates/nvs-db/src/pg.rs` searches
  for `concat!("fn ", "changed")`, the same bytes assembled from two literals that are not those
  bytes; `oid_constants` escapes only because it counts inside a slice its own body sits outside of,
  which is luck. [until: reviewed 2026-09-06]
