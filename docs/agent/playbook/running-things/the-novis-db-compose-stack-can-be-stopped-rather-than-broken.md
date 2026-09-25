- **The `novis-db` compose stack can be *stopped* rather than broken, and a `[1 floor]` check then
  names a database the program never opens.** The line is `warning: no queue worker started:
  [db.main] ... did not open`, about the first service `nvs.toml` tries. `docker ps -a --format
  "{{.Names}}\t{{.Status}}"` shows it: every container `Exited (255)` at one timestamp is a host
  restart, and `docker compose -f tests/db/compose.yaml up -d` restores it.
  [until: gone tests/db/compose.yaml:novis-db]
