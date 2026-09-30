Returns a new date with the year, the month or the day replaced.

You name only the parts you want to change, and the other parts stay as they are. The date you call
`with` on does not change.

The new parts must make a date that exists. 29 February 2024 with the year changed to 2023 is not
in the calendar, so `with` throws a `RuntimeError`. `plus` is different: it gives the last day of
the month in that case.

A common use is the first day of a month: replace the day with 1. One month later, minus one day,
is the last day of the same month.
