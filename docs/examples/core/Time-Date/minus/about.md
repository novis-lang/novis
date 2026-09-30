Returns a new date that is a number of days, weeks, months, quarters or years earlier.

You give a count and a unit from `Core\Unit`, such as `Core\Unit::Day`. The date you call `minus`
on does not change. It works the same way as `plus` in the other direction, and a negative count
gives a later date.

When the day does not exist in the month of the result, the result is the last day of that month.
31 March 2024 minus one month is 29 February 2024.

Use it to count back from a day. To check that a customer is at least 18 years old, go back 18
years from today and compare the birthday with that date.

**Good to know:** a unit smaller than a day, such as `Core\Unit::Hour`, throws a `LogicError`.
