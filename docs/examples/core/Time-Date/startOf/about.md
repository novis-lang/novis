Returns the first day of the week, the month, the quarter or the year that contains a date. You
choose the unit with a case of `Core\Unit`: `Unit::Week`, `Unit::Month`, `Unit::Quarter` or
`Unit::Year`.

A week starts on Monday, as in ISO 8601. A quarter is three months, so it starts on 1 January,
1 April, 1 July or 1 October. The result is a new `Date`, and the date you called it on does not
change.

A date has no time of day, so `Unit::Day` and smaller units do not compile.

`startOf(Unit::Week)` throws a `RuntimeError` only when the Monday is before the year `-9999`.

**Good to know:** `endOf` returns the last day of the same unit. The two together give the first and
the last day of a report.
