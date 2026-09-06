`deadline` covers the **whole call**: the connection, every redirect hop, every retry attempt and
every backoff between them. A single stated number is what a caller can reason about; the
per-attempt timeout most clients offer is the one that turns "5 seconds" into fifteen.

Two consequences are rules of their own. The deadline is never extended: when it would expire
during a backoff, the call throws immediately rather than sleeping and then failing, and a
`Retry-After` the server sent is clamped to the remaining deadline. And the total elapsed time of
a retried call does not exceed its deadline plus one connection timeout, which is the bound a
test asserts.

The redirect chain and every retry attempt share this one bound with the pinning rule they run
under: a hop is re-checked and re-pinned, a retry reuses the pinned address, and neither buys
itself more time (`rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`).
