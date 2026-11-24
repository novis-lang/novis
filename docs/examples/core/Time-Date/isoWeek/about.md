Returns the week number of a date in the ISO 8601 calendar, from `1` to `53`.

An ISO week starts on Monday and ends on Sunday. Week `1` is the week that contains the first
Thursday of the year. So the first days of January can be in week `52` or `53` of the year before,
and the last days of December can be in week `1` of the next year.

The year that a week belongs to is `isoWeekYear()`. It is different from `year()` for these few
days, so always print the week number together with `isoWeekYear()`. The result is a `uint`, and
`isoWeek` never throws an error.

**Good to know:** `format` with `YYYY` and `ww` prints the same year and week, such as
`format("YYYY-'W'ww")` for `2026-W53`.
