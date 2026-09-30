Tells you whether a rate-limited call was inside its limit.

`Core\RateLimit::shed` and `Core\RateLimit::consume` both return a `Core\RateLimit\Decision`. Its
`allowed` method returns `true` when the call was inside the limit. The units of the call are then
used. It returns `false` when the call was over the limit. A call that is not allowed uses no units,
so a later call with a smaller cost can still be allowed.

**Good to know:** a rate limit does not stop anything by itself. Your program reads `allowed` and
then does the work, or skips it and tells the user to try again later.
