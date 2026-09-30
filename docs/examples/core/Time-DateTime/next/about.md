Returns a new date and time on the next day that falls on a given weekday.

You give a case of `Core\Weekday`, such as `Core\Weekday::Monday`. The result is the nearest later
day with that weekday. The time of day and the time zone stay the same, and the value you call
`next` on does not change.

`next` always moves forward, by one to seven days. When the value is already on that weekday, the
result is seven days later.

Use it for work that repeats every week, such as a report that runs each Monday at 06:00. The
result keeps that time of day in a week when the clocks change.
