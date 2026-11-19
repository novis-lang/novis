Returns the hour of a time of day, as a `uint` from 0 to 23.

Midnight is 0, noon is 12 and 23:59 has hour 23. The value is the same number you give to
`TimeOfDay::at` as the first argument.

The result has the same name and type as the `hour` option of `with`. So
`$t->with({ hour: 8 })` gives the same minutes and seconds at 8 o'clock. The `hour` option must be
from 0 to 23, and a value outside that range throws a `RuntimeError`.

**Good to know:** to move the time forward and wrap past midnight, use `plus(1, Core\Unit::Hour)`.
`$t->with({ hour: $t->hour() + 1 })` throws a `RuntimeError` at hour 23.
