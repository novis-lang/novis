Returns the first date after this one that is on a given day of the week. You give the day as a
case of `Core\Time\Weekday`, such as `Weekday::Friday`.

The result is one to seven days later. When the date is already on that weekday, `next` returns
the same weekday one week later. To include the date itself, call `next` on the day before:
`$date->minus(1, Unit::Day)->next(Weekday::Friday)`.

The result is a new `Date`, and the date you called it on does not change. `next` throws a
`RuntimeError` only when the result is after the year `9999`.

**Good to know:** `previous` finds the weekday before a date.
