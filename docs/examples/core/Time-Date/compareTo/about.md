Compares two dates and tells you which one comes first in the calendar.

The result is `-1` when the date is earlier than the other one, `0` when both are the same day, and
`1` when it is later.

You can also write `<`, `>`, `<=`, `>=` and `<=>` between two dates. They all use `compareTo`.
`==` is `true` when both dates are the same day.

Use it to check a date against a limit, for example whether a coupon is still valid on the day of
an order. `Core\Arr::sort` needs a function that compares two values. Give it a function that
calls `compareTo`, and it sorts a list of dates from the earliest to the latest.
