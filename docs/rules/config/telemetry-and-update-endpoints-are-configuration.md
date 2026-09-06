The update-check URL and the telemetry upload URL have compiled-in defaults, overridable by environment
variable: `NVS_UPDATE_URL` and `NVS_TELEMETRY_URL`. Neither is a key in `nvs.toml` — they configure
the *tool*, not a deployment, and a serving process never uploads anything regardless of where the
endpoint points.

Configurability is what makes the rest of the telemetry design falsifiable. The test suite runs every
claim — the weekly upload gate, the three-second timeout, the never-uploads-from-serve rule, the exact
payload `nvs telemetry show` prints — against a local listener, and CI never touches the real host. The
consents, the counter schema and the two endpoints' contracts are the tooling chapter's.

**Not shipped.** Nothing in `crates/` reads either variable; implementation waits until the parity
chain's goals are done, and the reference says nothing of it until then.
