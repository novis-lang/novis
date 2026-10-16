Returns the last moment of the hour, day, week, month or year that a date and time is in.

You give a unit from `Core\Unit`. With `Core\Unit::Day` the result is 23:59:59.999999999 on the
same day. With `Core\Unit::Month` it is that time on the last day of the month. This is one
nanosecond before the next day or month starts. A week ends on Sunday. The result is in the same
time zone, and the value you call `endOf` on does not change.

Use it for a limit that includes the whole period, such as a sale that runs until the end of the
month.

**Good to know:** the last moment is not a full unit after the first one. On a day with 25 hours,
`difference` counts 24 whole hours from the `startOf` result to the `endOf` result.
