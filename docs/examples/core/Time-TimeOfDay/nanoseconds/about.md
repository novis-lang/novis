Returns the part of a second after the whole seconds, as a `uint` number of nanoseconds.

The result is from 0 to 999999999. One second has 1000000000 nanoseconds, so half a second is
`500000000`. A time made with `TimeOfDay::at` without a `nanoseconds` option has `0` here.

The result has the same name and type as the `nanoseconds` option of `with`. So
`$t->with({ nanoseconds: 0 })` gives the same time with the part of a second removed. The
`nanoseconds` option must be from 0 to 999999999, and a value outside that range throws a
`RuntimeError`.

**Good to know:** to show the part of a second, use `format`. Each `S` in the pattern prints one
digit of the fraction, so `SSS` prints milliseconds.
