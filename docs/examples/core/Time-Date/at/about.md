Creates a date from a year, a month and a day.

A `Core\Time\Date` is one day in the calendar. It has no time of day and no time zone, so it is the
right type for a birthday, a due date or a holiday.

`at` checks the three numbers. The month must be 1 to 12, and the day must exist in that month of
that year. When the numbers are not a date in the calendar, such as 30 February, `at` throws a
`RuntimeError`. You can catch that error to check a date that a person typed into a form.

**Good to know:** the year must be between -9999 and 9999. If you come from PHP, this check
replaces `checkdate`.
