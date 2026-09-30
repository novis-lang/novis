Returns how many units a key can still use after a rate-limited call.

The number is counted after the call itself. With a limit of 3 per minute, the first call gives `2`,
the second gives `1` and the third gives `0`. A result of `0` means the next call has to wait.

A call that is not allowed uses no units. With 6 units left, a call that costs 7 is not allowed, and
`remaining` is still `6`.

**Good to know:** units come back as time passes, so the number is correct only for the moment of
the call. Use it for a `RateLimit-Remaining` header or for a message such as "2 exports left today".
