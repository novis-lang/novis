`Core\Http\Client` sends `traceparent` on every outbound call while `[trace] propagate` is on, which
is what makes a trace cross a service boundary at all. `propagate` ships **on**
(`rule:observability/metrics-and-trace-blocks-are-system`); off, no header leaves.

What leaves is the runtime's own id — one per request, from
`rule:observability/a-trace-id-exists-for-every-request` — never one minted per call. Exactly one
`traceparent` is sent: a caller that wrote its own header keeps it, because a second line beside it
is what the W3C format tells a receiver to read as no header at all, and that would end the trace at
this hop while looking right on the line that sent ours.
