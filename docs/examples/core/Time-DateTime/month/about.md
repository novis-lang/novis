Returns the month of a date and time, as a `uint` from 1 to 12.

January is 1 and December is 12. The month is read in the time zone of the value. At 23:30 UTC on
31 March, `month` returns `3` in UTC and `4` in Tokyo.

The result has the same name and type as the `month` option of `with`. So
`$d->with({ month: $d->month() + 1 })` gives the same day in the next month. The `month` option
must be from 1 to 12. In December, `$d->month() + 1` is 13, and `with` throws a `RuntimeError`.
`with` also throws when the day does not exist in the new month, such as 31 April.

**Good to know:** `plus(1, Core\Unit::Month)` also moves to the next month, and it goes from
December to January of the next year.
