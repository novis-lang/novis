The service that receives telemetry is `apps/telemetry/`, a Novis web application in this repository,
in the shape of `apps/notes/`. It runs at `telemetry.novis-lang.org`, and
`https://telemetry.novis-lang.org/v1/report` is the default of `NVS_TELEMETRY_URL` once it is deployed
(`rule:config/telemetry-and-update-endpoints-are-configuration`). It does four things and nothing else:
it checks a report (`rule:tooling/a-telemetry-report-is-checked-against-a-closed-schema`), adds it to
the weekly totals (`rule:tooling/the-collector-keeps-weekly-totals-only`), keeps every address off disk
(`rule:tooling/the-collector-never-stores-an-address`), and publishes all it has
(`rule:tooling/telemetry-totals-are-public`).

The code is public because the repository is. The site's footer names the commit it was deployed
from, linked to that commit's tree, so a reader can read the exact code that answered. The deploy guide
in `apps/telemetry/README.md` is the whole deployment: the `nvs.toml`, the TLS proxy's configuration
with no log, and how the commit is recorded. Nothing about the deployment is private except the host's
own credentials.

The collector is a Novis program because it is small, its job is a web request, and running it is a
real-world test of the language under traffic it does not control. It shares no code, database or
process with the registry, so a fault in one never reaches the other.

- **Memory:** per request, one report of at most 4 KiB and one transaction. The rate limit's table is
  per process (`rule:tooling/the-collector-never-stores-an-address`).

**Not on disk.** There is no `apps/telemetry/`, and `NVS_TELEMETRY_URL` has no default.

**Declined:**
- **A hosted analytics product or a third-party collector.** It moves the promise to a company whose
  code nobody here can read, and adds a second party to every report.
- **A separate repository.** The known command list must follow the binary's command table, and a
  test can check that only when both are in one tree.
- **Sharing the registry's process or database.** Two services with one fault domain, for no gain.
