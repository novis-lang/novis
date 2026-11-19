Returns the day of the week of a date, as a case of the `Core\Time\Weekday` enum.

The result is one of seven cases, from `Core\Time\Weekday::Monday` to
`Core\Time\Weekday::Sunday`. You compare it with `==`. A date has no time zone, so the
result is the same everywhere.

`as int` converts the case to its position in the week, and Monday is 0. Saturday is 5 and Sunday
is 6. So a date is a working day from Monday to Friday when this number is less than 5.

**Good to know:** to print the name of the day, use `format` with `EEEE`.
