- **A `tests/db/compose.yaml` comment is a claim about the image, not about the service as
  configured.** MySQL generates a CA in its data directory at first boot, MariaDB's own certificate
  has no `subjectAltName`, and SQL Server keeps its certificate inside the instance rather than on
  the filesystem, and each of these has changed between release lines. Ask the running container
  what it serves — `docker compose exec <svc>` — before writing anything that verifies a certificate
  against it. `until: gone tests/db/compose.yaml:auto-generated leaf`
