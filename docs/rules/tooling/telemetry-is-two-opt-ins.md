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

The verb is `update-check`, not `update`, on purpose: `nvs update` is the package resolver's, and the two
never collide. Opt-out telemetry gives bigger numbers and is the wrong default for a runtime whose first
priority is that operators can trust it; the undercount is accepted and stated, with download counts and
cronned update checks as the correction.

**Not shipped.** Nothing in `crates/` implements any of this; the implementing slices are scheduled after
the parity chain's goals reach acceptance, and the reference says nothing of it until then.
