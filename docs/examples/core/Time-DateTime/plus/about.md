Returns a new date and time that is a number of units later, such as 3 days or 2 months.

You give a count and a unit from `Core\Unit`, such as `Core\Unit::Hour`. The value you call `plus`
on does not change, and the result is in the same time zone. A negative count gives an earlier
value.

When you add months, quarters or years, the day of the month stays the same if the new month has
that day. If it does not, the result is the last day of that month. 31 January 2024 plus one month
is 29 February 2024.

**Good to know:** one day is not always 24 hours. On a day when the clocks change, the day has 23
or 25 hours. Adding one day keeps the time of day. Adding 24 hours can give a different time of
day.
