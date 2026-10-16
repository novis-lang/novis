Limits how often something may happen for one key, counting in this server process only.

A key is what the limit is for, such as one IP address or one user. You say how many units the key may
use in a period, such as 2 per minute. Each call uses one unit, or the number you give as `cost`. The
result is a `Core\RateLimit\Decision`. It says if the call was allowed, how many units are left, and how
long to wait.

The count is in the memory of this server process, so `shed` is fast and needs no network store. Two
server processes with a limit of 100 allow up to 200. The count can also be forgotten when the server
needs the memory for other values. Use `shed` to protect a server from too much load. For a limit you
promised to a customer, use `Core\RateLimit::consume`.

**The examples below** show a fourth call that is not allowed, then a call that costs several units,
then a slow page that returns status 429 and a `Retry-After` header.
