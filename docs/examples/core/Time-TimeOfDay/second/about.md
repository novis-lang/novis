Returns the second of a time of day, as a `uint` from 0 to 59.

The second is a whole number. The part of the second after it is returned by `nanos`. A time made
with `TimeOfDay::at` without a `second` option has second 0.

The result has the same name and type as the `second` option of `with`. So
`$t->with({ second: 0 })` gives the start of the same minute. The `second` option must be from 0
to 59, and a value outside that range throws a `RuntimeError`.

**Good to know:** `$t->hour() * 3600 + $t->minute() * 60 + $t->second()` gives the seconds since
midnight.
