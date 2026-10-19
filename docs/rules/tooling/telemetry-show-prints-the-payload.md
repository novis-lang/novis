`nvs telemetry show` prints the exact payload the next upload would send, before anything is sent, and its
bytes equal what the listener then receives. That is what makes the design falsifiable rather than
promised: a reader can compare the printed payload against the closed counter schema
(`rule:tooling/telemetry-counters-are-a-closed-set`) with no trust in prose.

The client side is published when this ships. The reference's tools chapter documents both consents,
the complete counter list and both endpoints' contracts, and that chapter is the user-facing statement
of what is collected. The service side is `apps/telemetry/`
(`rule:tooling/the-telemetry-collector-is-a-novis-program`): it keeps weekly totals only
(`rule:tooling/the-collector-keeps-weekly-totals-only`), stores no address
(`rule:tooling/the-collector-never-stores-an-address`), and publishes every total
(`rule:tooling/telemetry-totals-are-public`), so anyone can see exactly what the project sees.

**Not on disk.** There is no `nvs telemetry show`.
