Returns a new date that is a number of days, weeks, months, quarters or years later.

You give a count and a unit from `Core\Unit`, such as `Core\Unit::Week`. The date you call `plus`
on does not change. A negative count gives an earlier date.

When you add months, quarters or years, the day of the month stays the same if the new month has
that day. If it does not, the result is the last day of that month. 31 January 2024 plus one month
is 29 February 2024.

**Good to know:** a date has no time of day. A unit smaller than a day, such as
`Core\Unit::Minute`, throws a `LogicError`.
