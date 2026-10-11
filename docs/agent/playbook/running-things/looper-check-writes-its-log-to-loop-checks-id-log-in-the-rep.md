- **`looper check` writes its log to `.loop/checks/<id>.log` in the repository, not to the loop's
  `var/loops/<n>/checks/` path the run header names.** That other log is the last sweep's, so reading
  it after a local re-run shows the old failure, and the run looks unchanged. Read the path the
  `--json` form prints (`"log"`). `bun nv suite` itself prints only `GREEN` and one `(pass) <path>`
  line per `--case`. See `tools/nv/cmd/suite.ts`.
