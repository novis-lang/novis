Returns the ISO 8601 week number of this date and time, from `1` to `53`.

An ISO week starts on Monday and ends on Sunday. Week `1` is the week with the first Thursday of
the year. Many calendars and business reports in Europe use these week numbers.

The first days of January can be in week `52` or `53` of the year before. The last days of December
can be in week `1` of the next year. `isoWeekYear` returns the year that the week belongs to, so use
the two together.

The date is the date in the value's own time zone. The pattern letter `w` in `format` prints the
same week number.
