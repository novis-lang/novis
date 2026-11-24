Returns the number of days in the month of a date, from `28` to `31`.

February has `29` days in a leap year and `28` days in other years. The result depends only on the
year and the month, so every day of a month gives the same number.

Use it to loop over every day of a month, or to share a monthly amount over its days. The result
is a `uint`, and `daysInMonth` never throws an error.

**Good to know:** `endOf(Unit::Month)` returns the last day of the month as a `Date`.
