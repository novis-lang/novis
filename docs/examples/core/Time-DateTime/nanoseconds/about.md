Returns the part of a second after the whole seconds, as a `uint` number of nanoseconds.

The result is from 0 to 999999999. One second has 1000000000 nanoseconds, so half a second is
`500000000`. A time from `Core\Time::at` without a `nanoseconds` option has `0` here. Like the other
readers, `nanoseconds` reads the time in the time zone of the value. The part of a second is the same in
every zone.

The result has the same name and type as the `nanoseconds` option of `with`. So
`$d->with({ nanoseconds: 0 })` gives the same time with the part of a second removed.

**Good to know:** to show the part of a second, use `format` with `S`. `SSS` prints milliseconds.
