- **A `tests/db/compose.yaml` comment is a claim about the image, not about the service as
  configured.** MariaDB 11.4 does not turn TLS on by itself, MySQL 8.4 generates a CA at
  `/var/lib/mysql/ca.pem`, and SQL Server keeps its certificate inside the instance rather than on
  the filesystem. Ask the running container what it serves — `docker compose exec <svc>` — before
  writing anything that verifies a certificate against it. [until: reviewed 2026-09-06]
