Returns a `Core\Time\DateTime` at the first moment of the date in the time zone you give.

A `Core\Time\Date` has no time of day and no zone. `atStartOfDay` adds both: the result is the
same date in that zone, at the start of the day. In most zones, on most days, this is midnight.

Some zones change the clock at midnight for summer time. On that day, the clock goes from 23:59
straight to 01:00, so 00:00 does not exist. Then the result is the first moment that exists, such as
01:00.

To get another time on the same date, call `withTime` on the result with a
`Core\Time\TimeOfDay`, such as `Core\Time\TimeOfDay::at(9, 30)`.

**Good to know:** the same date starts at a different moment in each zone.
