`nvs telemetry show` prints the exact payload the next upload would send, before anything is sent, and its
bytes equal what the listener then receives. That is what makes the design falsifiable rather than
promised: a reader can compare the printed payload against the closed counter schema
(`rule:tooling/telemetry-counters-are-a-closed-set`) with no trust in prose.

Both sides are published when this ships. The reference's tools chapter documents both consents, the
complete counter list and both endpoints' contracts, and that chapter — not a design record — is then the
user-facing statement of what is collected. The service side's contract is published with it: aggregates
only, no IP retention, and the aggregated data itself public, so anyone can see exactly what the project
sees.
