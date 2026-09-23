- **A floor check that passed for sessions can go red because the Docker daemon restarted without
  the test servers, and the line the ledger prints names something else.** `examples/queue.nvs`
  opens its stderr with a `warning[W1008]` about an ungranted `cache.shared` store, while the
  failure is two lines below it: `[db.main]` at `127.0.0.1:15432` refusing the connection. Run
  `docker ps` before reading a green-yesterday check as a regression, and `docker compose -f
  tests/db/compose.yaml up -d --wait postgres redis` brings the two servers back.
  [until: reviewed 2026-09-13]
