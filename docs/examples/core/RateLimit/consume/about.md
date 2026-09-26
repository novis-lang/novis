Limits how often something may happen for one key, and gives the same answer on every server.

A key is what the limit is for: one account, one API key, one IP address. You say how many units the
key may use in a period, such as 5 per 15 minutes. Each call uses one unit, or the number you give as
`cost`. The result is a `Core\RateLimit\Decision`. It says whether the call was allowed, and how long
to wait when it was not.

The count is kept in the shared store that the person who runs the server sets up, so every core and
every machine counts together. When that store cannot be reached, `consume` throws an error. Your
program then decides what that means: refuse, for a login limit, or allow, for a plan quota.

**Good to know:** `Core\RateLimit::shed` takes the same arguments and counts on one core only. It
needs no store, and its limit is less exact.

**The examples below** show the settings `consume` needs, then a login limit that refuses when the
store is away, then an API quota that writes response headers.
