Returns the hour of a date and time, as a `uint` from 0 to 23.

Midnight is 0 and 23:00 is 23. The hour is read in the time zone of the value. At 08:00 UTC,
`hour` returns `9` in Berlin, `3` in New York and `17` in Tokyo.

The result has the same name and type as the `hour` option of `with`. So
`$d->with({ hour: $d->hour() + 1 })` gives the same day one hour later on the clock. The `hour`
option must be from 0 to 23, so at hour 23 this throws a `RuntimeError`.

**Good to know:** to show a time to a user, first convert it to the user's zone with `in($zone)`.
Then read the hour.
