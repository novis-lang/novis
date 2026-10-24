`POST /v1/report` takes one `application/json` body of at most 4 KiB, the exact bytes `nvs telemetry
show` printed (`rule:tooling/telemetry-show-prints-the-payload`), in the shape
`rule:tooling/telemetry-counters-are-a-closed-set` gives. The request carries no cookie, no token and
no identifier, and the answer sets none.

The collector checks every field, and one wrong field refuses the whole report:

| Field | Accepted |
|---|---|
| `schema` | `1` |
| `version` | `MAJOR.MINOR.PATCH` with an optional `-` pre-release of `[0-9a-z.]`, at most 32 bytes |
| `os`, `arch` | a value from `apps/telemetry/known.toml` |
| `week` | `YYYY-Www`, a week that ended by the collector's clock, with one day of tolerance, and is at most four weeks old |
| `first` | `true` or `false` |
| `days_active` | an integer from 1 to 7 |
| `counters` | at most 64 keys, each `invocation.<name>` or `internal-error.<name>` with `<name>` matching `[a-z][a-z0-9-]{0,31}`, each value an integer from 0 up |

A key that matches the grammar but names a command `known.toml` does not list is counted as
`invocation.unknown` or `internal-error.unknown`. So a made-up name never reaches the public pages,
and a report from a newer `nvs` still counts. A value above 100 000 is counted as 100 000: a large
honest week is kept, and one report cannot move a total by more than that. A test in the repository
checks that `known.toml` lists every command in the binary's table, so the list cannot fall behind.

The answers, and what the client does with each
(`rule:tooling/a-failed-upload-never-fails-the-command`):

| Status | Meaning | The client |
|---|---|---|
| `204` | added to the totals | records the upload |
| `400` | a field is wrong; the body names it | drops that week's report |
| `413` | the body is over 4 KiB | drops that week's report |
| `415` | not `application/json` | drops that week's report |
| `429` | the address sent too many reports; `Retry-After` says when | keeps it for next time |
| `5xx` | the collector failed | keeps it for next time |

A report the client sent twice, because its three-second budget ran out after the collector had
already answered `204`, is counted twice. That case is rare, the cost is one report, and preventing it
needs an identifier on every report.

**Not on disk.** The collector never answers `429`, and no test holds `known.toml` to the binary's
command table.

**Declined:**
- **Refusing a report over a value cap.** It loses an honest heavy week, and the cap already bounds
  what one report can move.
- **A signed report or a per-install key.** A key is an identifier, and the anonymity is worth more
  than the spam it would stop. The rate limit and the caps bound the damage
  (`rule:tooling/the-collector-never-stores-an-address`).
- **Accepting any version string.** A free-text field would put whatever a sender writes on a public
  page.
