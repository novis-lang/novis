Returns the exact time between an earlier instant and this one, as a `Duration`.

`$end->since($start)` is how long it took to get from `$start` to `$end`. The result is an exact
number of nanoseconds. It has no months or days of different lengths, so it is the same number
wherever you run the program. When the argument is after this instant, the result is negative.
So `$start->plus($end->since($start))` is always `$end` again.

A `Duration` can be at most about 292 years long. When the two instants are further apart than
that, `since` throws a `RuntimeError`.

To count calendar days or months between two dates, use a `DateTime` in a time zone instead.
