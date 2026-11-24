Returns the last day of the week, the month, the quarter or the year that contains a date. You
choose the unit with a case of `Core\Unit`: `Unit::Week`, `Unit::Month`, `Unit::Quarter` or
`Unit::Year`.

A week ends on Sunday, as in ISO 8601. `endOf(Unit::Month)` knows the length of each month, so it
returns `2024-02-29` in a leap year and `2023-02-28` in other years. The result is a new `Date`,
and the date you called it on does not change.

A date has no time of day, so `Unit::Day` and smaller units do not compile.

`endOf(Unit::Week)` throws a `RuntimeError` only when the Sunday is after the year `9999`.

**Good to know:** `startOf` returns the first day of the same unit.
