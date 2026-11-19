Returns the year of a date and time, as an `int`.

The year is read in the time zone of the value. One moment can be in two different years in two
zones. At 23:30 UTC on 31 December 2024, `year` returns `2024` in London and `2025` in Berlin.

The result has the same name and type as the `year` option of `with`. So
`$d->with({ year: $d->year() + 1 })` gives the same date and time one year later. It throws a
`RuntimeError` when that date does not exist, such as 29 February in a year that is not a leap year.

**Good to know:** to read the year in another zone, first convert the value with
`toInstant()->in($zone)`.
