Returns the last date before this one that is on a given day of the week. You give the day as a
case of `Core\Time\Weekday`, such as `Weekday::Monday`.

The result is one to seven days earlier. When the date is already on that weekday, `previous`
returns the same weekday one week earlier. To include the date itself, call `previous` on the day
after: `$date->plus(1, Unit::Day)->previous(Weekday::Monday)`.

The result is a new `Date`, and the date you called it on does not change. `previous` throws a
`RuntimeError` only when the result is before the year `-9999`.

**Good to know:** `next` finds the weekday after a date.
