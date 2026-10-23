The update check reads the latest release from the GitHub Releases API of the public repository
(`https://github.com/novis-lang/novis`), and the telemetry upload has no default URL. The environment
variables `NVS_UPDATE_URL` and `NVS_TELEMETRY_URL` set either one. Neither is a key in `nvs.toml`:
they configure the *tool*, not a deployment, and a serving process never uploads anything regardless
of where the endpoint points.

With no `NVS_TELEMETRY_URL`, telemetry is complete on the local side and uploads nothing. `nvs telemetry
on` records counters and says that nothing is uploaded until a collector exists. No collector exists,
and a default URL would send data to a host nobody runs. The release list is already public and served
by GitHub, so the update check needs no Novis-run service.

Configurability is what makes the rest of the telemetry design falsifiable. The test suite runs every
claim — the weekly upload gate, the three-second timeout, the never-uploads-from-serve rule, the exact
payload `nvs telemetry show` prints — against a local listener, and CI never touches a real host. The
consents, the counter schema and the two endpoints' contracts are the tooling chapter's.

- **Memory:** none on the request path. The update check reads one small JSON document per run.

The default is `https://api.github.com/repos/novis-lang/novis/releases/latest`. A variable that is
empty or only whitespace counts as unset. Each is read when a command needs it and never kept, and
`nvs --help` lists both.

**Declined:**
- **A default telemetry upload URL before a collector exists.** It sends opted-in data to a host nobody
  runs. A collector is its own decision, and it sets the default when it lands.
- **A Novis-run update endpoint.** GitHub's Releases API already serves the release list of the public
  repository, and a second service is a second thing to secure.
