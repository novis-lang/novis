Limits how often something may happen for one key, counting in this server process only.

A key is what the limit is for: one IP address, one user, one slow page. You say how many units the
key may use in a period, such as 2 per minute. Each call uses one unit, or the number you give as
`cost`. The result is a `Core\RateLimit\Decision`. It says whether the call was allowed, how many units
are left, and how long to wait when the call was not allowed.

`shed` keeps its count in the memory of this server process, and every core counts together. It needs
no shared store, so it is fast and never fails because a store cannot be reached. The limit is per
process: two server processes with a limit of 100 allow up to 200. The count can also be forgotten when
memory is needed for other data. Use `shed` to protect a server from too much load. When the limit is a
promise to a customer, use `Core\RateLimit::consume`.

**The examples below** show a fourth call that is not allowed, then a call that costs several units,
then a slow page that returns status 429 and a `Retry-After` header.
