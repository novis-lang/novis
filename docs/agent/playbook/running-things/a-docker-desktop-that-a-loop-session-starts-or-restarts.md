- **A Docker Desktop that a loop session starts or restarts stops when that session's turn ends, and
  the next sweep fails with `[db.main]` at `127.0.0.1:15432` refusing the connection.** On Windows each
  process a turn starts is in a job that kills every member when the turn exits, and the driver's
  session job keeps the session's whole tree inside it, so `docker desktop start` from a session only
  lasts until that sweep. Leave the daemon to the driver: `preflight` in `tools/nv/driver/gates.ts`
  runs on every turn and starts Docker Desktop detached when `docker info` fails.
  [until: gone tools/nv/driver/gates.ts:detached]
