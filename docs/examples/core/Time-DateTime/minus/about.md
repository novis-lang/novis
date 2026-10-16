Returns a new date and time that is a number of units earlier, such as 3 days or 2 months.

You give a count and a unit from `Core\Unit`, such as `Core\Unit::Week`. The value you call `minus`
on does not change, and the result is in the same time zone. It works the same way as `plus` in the
other direction, and a negative count gives a later value.

When the day does not exist in the month of the result, the result is the last day of that month.
31 March 2024 minus one month is 29 February 2024.

Use it to count back from a fixed time, for example to send a reminder one day before an
appointment.

**Good to know:** going back one day keeps the time of day, also when the clocks change on that
day. Going back 24 hours can give a different time of day.
