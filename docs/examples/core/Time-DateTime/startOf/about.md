Returns the first moment of the hour, day, week, month or year that a date and time is in.

You give a unit from `Core\Unit`. With `Core\Unit::Day` the result is the start of that day. With
`Core\Unit::Month` it is the start of the first day of the month. A week starts on Monday. A
quarter is three months, and the quarters start in January, April, July and October. The result is
in the same time zone, and the value you call `startOf` on does not change.

Use it to find the period that a value belongs to, for example the month that a report covers.

**Good to know:** a day usually starts at 00:00, but not always. In some time zones the clocks move
forward at midnight. On that day the first moment is 01:00.
