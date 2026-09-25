- **A `member` row in `docs/spec/02-php-migration.md` whose Novis cell names a class in prose alone is
  invisible to every walk over that table.** `migration_member_refs`'s second regex captures
  `Core\X::y` and nothing else, so the whole zlib group — fourteen `member` rows saying `Core\Compress`
  — read as covered while pinning no member at all, and both
  `every_migration_member_row_names_a_registered_member` and its conformance twin passed over it
  vacuously. Grep a family's rows for `::` before believing the table has them pinned, and spell the
  member into the cell in the slice that registers the class. [until: gone crates/nvs-stdlib/tests/spec_registry_coverage.rs:migration_member_refs]
