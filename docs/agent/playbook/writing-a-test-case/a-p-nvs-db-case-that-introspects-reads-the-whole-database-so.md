- **A `-p nvs-db` case that introspects reads the *whole* database, so an unrelated table can refuse
  the read.** `nvs_db::direct::schema_of` assembles every table the catalog answers and fails with
  `ReadError::Vocabulary` at the first column type the closed vocabulary cannot name — on the shared
  matrix server that was `nvs_jobs.script text`, written by a hand-written list in another crate, and
  the failure reads as the fixture being wrong. Narrow the *comparison* to the fixture's own tables,
  and ask `information_schema` which column the server means before blaming the round trip.
  [until: gone crates/nvs-db/src/direct.rs:ReadError::Vocabulary]
