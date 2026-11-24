Checks which of two date-times is earlier, and returns `-1`, `0` or `1`.

The result is `-1` when this date-time is earlier than the other one, `0` when both are the same
moment, and `1` when it is later. The check uses the moment, down to the nanosecond. The time zone
does not count. 10:00 in `Europe/Berlin` and 09:00 in `Europe/London` on the same winter day are
the same moment, so the result is `0`.

The operators `<`, `>`, `<=`, `>=` and `<=>` use `compareTo` too, so you can write `$a < $b`. `==`
also compares the moment, so it is `true` for the two values above.

**Good to know:** two values can be the same moment and still show different clock times. To check
that two values also have the same time zone, compare `zone()` as well.
