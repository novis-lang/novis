Returns the year that the ISO 8601 week of this date and time belongs to.

For most days, the result is the same as `year()`. It is different for a few days around New Year.
1 January 2027 is in week `53` of 2026, so `isoWeekYear()` returns `2026` and `year()` returns
`2027`. The last days of December can be in week `1` of the next year.

Use `isoWeekYear` together with `isoWeek`, for example to write a week as `2026-W53`. The date is the
date in the value's own time zone. The pattern letter `Y` in `format` prints the same year.
