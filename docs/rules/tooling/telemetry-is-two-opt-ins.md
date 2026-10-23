Adoption is measured by two independent consents, both off by default. Usage telemetry is switched by
`nvs telemetry on|off|status`; the update check runs only when invoked, `nvs update-check`, interactively
or from the operator's own cron. Neither switch ever implies the other: enabling update checks never
silently enables usage reporting, and vice versa. One TCP connection may carry both only when both are on.

Consent and counters live in plain files under the user's nvs data directory, inspectable with any editor.
`nvs telemetry off` stops recording, and `nvs telemetry reset` — or deleting the directory — wipes
everything held locally. What is recorded is a closed set of operator actions
(`rule:tooling/telemetry-counters-are-a-closed-set`), an upload is carried and never initiated
(`rule:tooling/a-serving-process-never-uploads`), and the exact payload is printable before anything is
sent (`rule:tooling/telemetry-show-prints-the-payload`).

The verb is `update-check`, not `update`, on purpose: updating a dependency is `nvs pkg update`
(`rule:packaging/package-toml-is-a-tool-input`), and a bare `nvs update` would read as either.

The population worth measuring is the developers and operators who run `nvs`, most of them as an HTTP
server. `nvs serve` is itself a command, so the boundary is not the CLI against the server: it is
operator actions against traffic, and local writes against network uploads. Download and pull counts
need no runtime code and keep working for hosts that never opt in; they are the floor this sits on.
The undercount is accepted and stated: opt-in telemetry reports a small share of the real population,
and a deployed server whose operator never runs another command records forever and uploads never.
Download counts and cronned update checks are the correction.

**The files.** Both are under the data folder's `telemetry/` directory, which only `nvs telemetry on`
creates, privately, after the data folder passes its ownership check. `telemetry/consent` is `on` or
`off`, and a folder without it is off. `telemetry/counters` holds the counters
(`rule:tooling/telemetry-counters-are-a-closed-set`). `off` writes `off` and keeps the counters, and
`reset` deletes the directory, which also turns telemetry off. `status` prints the switch, the counter
file's path, the upload URL or the note that `NVS_TELEMETRY_URL` is not set, and the lifetime upload
totals. `on` says that nothing is uploaded while no upload URL is set
(`rule:config/telemetry-and-update-endpoints-are-configuration`).

**`DO_NOT_TRACK` and `CI`.** A non-empty `DO_NOT_TRACK` or `CI` turns telemetry off for that process:
nothing is recorded and nothing is sent, the stored consent is left as it is, and `status` names the
variable. The update check still runs when it is invoked. An operator's standing opt-out wins over a
stored consent, and a CI machine's runs are not the population being measured.

**Not on disk.** Nothing uploads yet (`rule:tooling/a-serving-process-never-uploads`).

**Declined:**
- **Opt-out telemetry.** It gives bigger numbers and is the wrong default for a runtime whose first
  priority is that operators can trust it.
- **An `X-Powered-By`-style response header as measurement.** It measures nothing by itself, since only
  someone else's crawl counts it, and it discloses a version on every response to visitors who never
  consented. That spends security and request isolation for data the project never receives.
