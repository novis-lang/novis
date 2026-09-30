Returns the day of the week of a date and time, as a case of the `Core\Weekday` enum.

The result is one of seven cases, from `Core\Weekday::Monday` to `Core\Weekday::Sunday`. You
compare it with `==`, for example to check whether a delivery falls on a weekend.

`weekday` reads the day in the time zone of the value. One moment can be a Sunday in Los Angeles
and already a Monday in Tokyo.

**Good to know:** the result is not a number. If you need one, `as int` converts the case to its
position in the week, and Monday is 0. To print the name of the day, use `format` with `EEEE`.
