An upload has a **three-second budget** end to end. When it expires, or the connection fails, the attempt
is abandoned, the counters are kept for the next carrier, and the output says exactly that —
`telemetry: could not send (timeout after 3s); kept for next time`. The carrying command's own work and
exit status are untouched, and there is no retry inside the same invocation.

An operator who opted in is lending a command they ran for another reason
(`rule:tooling/a-serving-process-never-uploads`); an unreachable endpoint may cost them three seconds and
one honest line, never a failed cron job or a changed exit code
(`rule:tooling/update-check-states-the-whole-picture`).
