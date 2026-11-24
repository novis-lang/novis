Returns the second of a date and time, as a `uint` from 0 to 59.

The second is read in the time zone of the value. It is a whole number. The part of the second
after it is returned by `nanoseconds`.

The result has the same name and type as the `second` option of `with`. So
`$d->with({ second: 0, nanoseconds: 0 })` gives the start of the same minute. The `second` option must
be from 0 to 59, and a value outside that range throws a `RuntimeError`.

**Good to know:** a date and time made from a `TimeOfDay` without a `second` option has second 0.
