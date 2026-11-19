Returns the year of a date, as an `int`.

2024-03-15 returns `2024`. The year is the same number you give to `Core\Time\Date::at`, and the
same number `format` prints with `yyyy`.

The result has the same name and type as the `year` option of `with`. So
`$date->with({ year: $date->year() + 1 })` gives the same day one year later. It throws a
`RuntimeError` when that day does not exist, such as 29 February in a year that is not a leap year.

**Good to know:** use `year` when you need a number to calculate with. Use `format` when you need
text to show.
