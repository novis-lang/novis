Returns the minute of a time of day, as a `uint` from 0 to 59.

12:15 returns `15`, and a time on the hour returns `0`. The value is the same number you give to
`TimeOfDay::at` as the second argument.

The result has the same name and type as the `minute` option of `with`. So
`$t->with({ minute: 0 })` gives the start of the same hour. The `minute` option must be from 0 to
59, and a value outside that range throws a `RuntimeError`.

**Good to know:** `$t->hour() * 60 + $t->minute()` gives the minutes since midnight. Two of these
numbers give the minutes between two times on the same day.
