Returns the minute of a date and time, as a `uint` from 0 to 59.

The minute is read in the time zone of the value. Most zones are a whole number of hours from UTC,
so the minute is the same as in UTC. Some zones are not. At 10:15 UTC, `minute` returns `15` in
UTC and `45` in India, which is 5 hours and 30 minutes ahead.

The result has the same name and type as the `minute` option of `with`. So
`$d->with({ minute: 0 })` gives the start of the same hour. The `minute` option must be from 0
to 59, and a value outside that range throws a `RuntimeError`.

**Good to know:** `$d->minute() % 15` gives the minutes since the last quarter hour.
