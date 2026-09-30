Returns a new date and time with the time of day replaced and the date kept.

You give a `Core\Time\TimeOfDay`. The result has its hour, minute, second and nanoseconds. The date
and the time zone stay the same, and the value you call `withTime` on does not change.

Use it when you know the day and want a fixed time on that day, such as 08:00 for a reminder.
`with` can change the same parts, but there you name each one. `withTime` always replaces all four,
so the result has no seconds from the old time.

**Good to know:** when the clocks move forward, one hour of that day does not exist. If the new
time is in that hour, the result is one hour later.
