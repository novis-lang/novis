Returns the day of the month of a date, as a `uint` from 1 to 31.

2024-03-15 returns `15`. The first day of every month is 1. The last day is 28, 29, 30 or 31,
depending on the month and the year.

The result has the same name and type as the `day` option of `with`. So `$date->with({ day: 1 })`
gives the first day of the same month.

**Good to know:** a date is the last day of its month when the next day has `day` equal to 1.
