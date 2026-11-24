Returns the year that the ISO 8601 week of a date belongs to.

An ISO week starts on Monday, and week `1` is the week that contains the first Thursday of the
year. A week is never split over two years. So for a few days around New Year, `isoWeekYear()` is
different from `year()`. 1 January 2027 is in week `53` of `2026`, and 30 December 2024 is in week
`1` of `2025`. On every other day, the two are the same.

Use `isoWeekYear()` together with `isoWeek()`. A label made from `year()` and `isoWeek()` is wrong
for these days. The result is an `int`, and `isoWeekYear` never throws an error.

**Good to know:** `format` with `YYYY` prints the same year.
