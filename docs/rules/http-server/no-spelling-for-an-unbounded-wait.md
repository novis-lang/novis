`Core\Http\Client` has no spelling for "wait forever". Every bound in `Core\Http\Options` —
`deadline`, `connectTimeout`, `retryBackoff` — is a `Duration`, which has no infinite value
(`rule:types/duration`); there is no `deadline: null` and no `0` meaning unbounded; and a
call that omits the field inherits `[http.client] deadline` (shipped `30s`) or `connect_timeout`
(shipped `5s`) rather than removing the bound. An unbounded outbound call is therefore not
something a program can express, the same way a shell string is not something `Core\Process` can
express (`rule:core-classes/process-is-argv-only`): the guarantee comes from the absence of a
spelling, not from a check.

The bag is a closed set of keys, and the three retry keys — `retryAttempts`, `retryBackoff`,
`retryIdempotencyKey` — are flat rather than a nested `retry` shape, because a bag flattens to one
ABI argument per option and a bag nested inside one has nothing to flatten into
(`rule:core-api/shape-rules` R2). The prefix keeps the grouping legible at a call site.

Expiry throws `TimeoutError`, never a falsy return (`rule:core-api/failure-throws`), and the
deadline it reports is the one that covers the whole call
(`rule:http-server/one-deadline-covers-the-whole-call`).
