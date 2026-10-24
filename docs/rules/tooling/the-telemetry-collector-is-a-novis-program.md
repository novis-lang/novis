The service that receives telemetry is `apps/telemetry/`, a Novis web application in this repository,
in the shape of `apps/notes/`. **The program names no host.** Every link and every endpoint it prints
is built from the request's own scheme and host, so the same code answers on `127.0.0.1:8000` and on
any domain. The domain is deployment configuration: the TLS proxy's site name, and the client's
`NVS_TELEMETRY_URL` (`rule:config/telemetry-and-update-endpoints-are-configuration`). The client's
default URL stays empty until a collector is deployed, and setting it then is one constant in
`crates/nvs-cli/src/endpoints.rs`. It does four things and nothing else:
it checks a report (`rule:tooling/a-telemetry-report-is-checked-against-a-closed-schema`), adds it to
the weekly totals (`rule:tooling/the-collector-keeps-weekly-totals-only`), keeps every address off disk
(`rule:tooling/the-collector-never-stores-an-address`), and publishes all it has
(`rule:tooling/telemetry-totals-are-public`).

The code is public because the repository is. The site's footer names the commit it was deployed
from, linked to that commit's tree, so a reader can read the exact code that answered. The deploy guide
in `apps/telemetry/README.md` is the whole deployment: the `nvs.toml`, the TLS proxy's configuration
with no log, and how the commit is recorded. Nothing about the deployment is private except the host's
own credentials.

**The whole service runs and is tested on one machine.** An end-to-end test starts `nvs serve` on the
application with a temporary database on loopback, points the real client at it through
`NVS_TELEMETRY_URL`, uploads a finished week with `nvs telemetry upload`, and reads the result back
from `/data/totals.json` and the pages. No test and no check reaches a real host. It is the one
test of the app under `nvs serve`. With no `[capabilities.fs] read`, `nvs test` reads any file and a
request reads only `public/`, so the app's `read` names `known.toml` and `REVISION`, the two files
outside `public/` it reads, and this test fails on a read the list leaves out.

The collector is a Novis program because it is small, its job is a web request, and running it is a
real-world test of the language under traffic it does not control. It shares no code, database or
process with the registry, so a fault in one never reaches the other.

- **Memory:** per request, one report of at most 4 KiB and one transaction. The rate limit's table is
  per process (`rule:tooling/the-collector-never-stores-an-address`).

**Declined:**
- **A hosted analytics product or a third-party collector.** It moves the promise to a company whose
  code nobody here can read, and adds a second party to every report.
- **A separate repository.** The known command list must follow the binary's command table, and a
  test can check that only when both are in one tree.
- **Sharing the registry's process or database.** Two services with one fault domain, for no gain.
- **A domain written into the program or its `nvs.toml`.** The program would then answer correctly on
  one host only, and a local run would print links to a site that is not the one answering.
