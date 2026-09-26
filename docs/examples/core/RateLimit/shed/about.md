Limits how often something may happen for one key, counting on this core only.

A key is what the limit is for: one IP address, one user, one slow page. You say how many units the
key may use in a period, such as 2 per minute. Each call uses one unit, or the number you give as
`cost`. The result is a `Core\RateLimit\Decision`. It says whether the call was allowed, how many units
are left, and how long to wait when the call was refused.

`shed` keeps its count in the memory of the core that runs the request. It needs no shared store, so
it is fast and never fails because a store is away. The limit is per core: a limit of 100 on a server
with 8 cores allows up to 800. The count can also be forgotten when memory is needed for other data.
Use `shed` to protect a server from too much load. When the limit is a promise to a customer, use
`Core\RateLimit::consume`.

**The examples below** show a fourth call being refused, then a call that costs several units, then a
slow page that answers with status 429 and a `Retry-After` header.
