Returns `true` when the year of the date is a leap year, and `false` when it is not.

A leap year has 366 days, because February has a 29th day. A year is a leap year when it divides
by 4. A year that divides by 100 is not a leap year, unless it also divides by 400. So 2024 and 2000
are leap years, and 1900 and 2023 are not.

The result depends only on the year. Every date in 2024 returns `true`.

**Good to know:** to check a year without a date, build any date in it, such as 1 January.
