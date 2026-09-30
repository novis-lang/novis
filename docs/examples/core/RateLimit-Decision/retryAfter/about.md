Returns how long to wait before a call that was not allowed can be made again.

The result is `null` when the call was allowed. When the call was not allowed, the result is a
`Core\Time\Duration`. It is the exact time until the same call would be allowed, so a client that
waits this long does not have to guess.

Use it to write a `Retry-After` header, or to tell a user how many minutes to wait. The header
needs whole seconds. Round the duration up, so that a client does not come back a moment too early.

**Good to know:** the wait is calculated for each call. Two clients that were stopped at different
moments get different wait times, so they do not all come back at the same moment.
