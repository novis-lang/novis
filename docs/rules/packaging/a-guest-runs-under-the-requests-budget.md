Guest execution is tied to the request's own limits. Epoch interruption binds it to the per-request
CPU cap, so a deliberately infinite guest loop traps rather than hanging a core; memory is capped
through the store's limits. An extension therefore cannot starve its neighbours — a stronger guarantee
than a built-in native function currently has, because it is enforced by the sandbox rather than by
discipline.

A trapped guest is a resource-limit failure of the request that called it, and reaches that request
the way any other limit does (`rule:errors/on-limit`); it never reaches the process. Together with
`rule:packaging/a-fresh-instance-per-request`, this is what makes a single-process server defensible
with third-party code inside it: a crashing or malicious extension harms one request, not every request
in flight.
