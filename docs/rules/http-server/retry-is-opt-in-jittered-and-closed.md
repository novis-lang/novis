`retryAttempts` absent means one attempt. Present, it is the **total** number of attempts
including the first, and must be at least 1. `retryBackoff` is the base delay, shipped `100ms`,
growing exponentially per attempt with **full jitter** — the actual wait is uniformly random in
`[0, base × 2^n]`. Jitter is not optional and not configurable: unjittered retries from many
hosts synchronise into a burst against a service that is already failing, which is the failure
mode retry is supposed to relieve.

What is retried is a closed set: a connection failure, a timeout, and status `429`, `502`, `503`
and `504`. Nothing else — a `400` or a `403` is an answer, and retrying it is a load generator; a
`500` is usually a real application error and is deliberately not on the list. A `Retry-After`
header on a `429` or `503` replaces the computed backoff in either of its forms, delay-seconds or
an HTTP-date, clamped to the remaining deadline — and a date already past keeps the jittered
backoff rather than becoming a zero wait, so the clients one date was handed to do not retry in
step.

Every attempt runs under the one deadline (`rule:http-server/one-deadline-covers-the-whole-call`)
and reuses the `Core\Http\Target` the launderer pinned, so a retry performs no second resolution
(`rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`). A `POST` or `PATCH` is not
retried at all without a key (`rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`).
