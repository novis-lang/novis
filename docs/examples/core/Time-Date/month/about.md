Returns the month of a date, as a `uint` from 1 to 12.

January is 1 and December is 12. 2024-03-15 returns `3`. To print the name of the month, use
`format` with `MMMM`.

The result has the same name and type as the `month` option of `with`. So
`$date->with({ month: $date->month() + 1 })` gives the same day in the next month. It throws a
`RuntimeError` when that day does not exist. `plus` with `Core\Unit::Month` gives the last day of
the month in that case.

**Good to know:** month 13 does not exist, so `with` also throws an error for a date in December.
