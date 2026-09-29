Creates a duration of a given number of days. Each day is exactly 24 hours.

Use it when the number of days is known only while the program runs, for example when it comes from
a setting. When the number is fixed, write a literal such as `30d` instead. A negative number gives
a negative duration. The number can be up to 106,751 days, which is about 292 years. A larger number
throws a `RuntimeError`.

**Good to know:** a calendar day is not always 24 hours. On the day the clocks change for daylight
saving time, it has 23 or 25 hours. To move a date to the same time on the next calendar day, use
`plus` on a `Core\Time\DateTime` with `Core\Unit::Day`.
