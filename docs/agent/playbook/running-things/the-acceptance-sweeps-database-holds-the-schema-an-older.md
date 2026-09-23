- **The acceptance sweep's database holds the schema an older iteration built, so a schema change
  fails the sweep on state and not on code.** Converging `[db.main]` onto the queue's new value
  needed `--including-risky` for two narrowed columns, and the old partial index `nvs_jobs_dedupe`
  collided by name because a construct outside the vocabulary is invisible to introspection rather
  than reported as a difference. Converge the container by hand — `docker exec
  novis-db-postgres-1 psql -U novis -d novis_test` — before reading a red check as a bug.
  [until: reviewed 2026-09-07]
