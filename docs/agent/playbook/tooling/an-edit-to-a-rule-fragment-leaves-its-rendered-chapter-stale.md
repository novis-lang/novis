- **An edit to a rule fragment under `docs/rules/<topic>/` leaves its chapter `docs/rules/<topic>.md`
  stale until `bun nv rules --render` runs.** A goal's last session moved its decisions into a rule's
  fragment and wrote DONE. Its `ci-check` then failed on `rulebook pages: red`, and the retire was
  refused. Run `bun nv rules --render` after every fragment edit, and commit the chapter with it.
  See `tools/nv/cmd/rules.ts`.
