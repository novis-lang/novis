`Core\Unit` is a step in the calendar or on the clock: a nanosecond, a microsecond, a millisecond, a
second, a minute, an hour, a day, a week, a month, a quarter or a year.

You give a `Unit` to the methods that move a date forward or back, such as `plus` and `minus`. You
also give one to `startOf` and `endOf`, and to `difference`, which counts the units between two
dates. The cases are in order from the smallest to the largest. A week starts on Monday.

**Good to know:** a calendar step is not always the same length of time. One month after 31 January
is the last day of February, because February has no day 31. In a time zone that moves its clocks
forward, that day has 23 hours. For a length of time that never changes, such as a timeout, use a
`Core\Time\Duration`.
