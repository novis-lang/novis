An upload has a **three-second budget** end to end. When it expires, or the connection fails, the attempt
is abandoned, the counters are kept for the next carrier, and the output says exactly that —
`telemetry: could not send (timeout after 3s); kept for next time`. The carrying command's own work and
exit status are untouched, and there is no retry inside the same invocation.

The collector's answer decides what happens to the week's report
(`rule:tooling/a-telemetry-report-is-checked-against-a-closed-schema`). A `429` or a `5xx` keeps it,
the same as a timeout. A `400`, `413` or `415` drops it, because the same bytes would be refused again,
and the line says so: `telemetry: refused by the collector (400: week); dropped`. Any other status is
treated as a `5xx`.

An operator who opted in is lending a command they ran for another reason
(`rule:tooling/a-serving-process-never-uploads`); an unreachable endpoint may cost them three seconds and
one honest line, never a failed cron job or a changed exit code
(`rule:tooling/update-check-states-the-whole-picture`).

**Not on disk.** Nothing in `crates/` uploads telemetry, so there is no upload to fail.
