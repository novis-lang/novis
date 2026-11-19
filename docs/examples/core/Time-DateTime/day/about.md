Returns the day of the month of a date and time, as a `uint` from 1 to 31.

The day is read in the time zone of the value. At 02:30 UTC on 6 March 2024, `day` returns `6` in
UTC and `5` in New York, where it is still the evening before.

The result has the same name and type as the `day` option of `with`. So `$d->with({ day: 1 })`
gives the first day of the same month, at the same time. `with` throws a `RuntimeError` when the
day does not exist in that month, such as 31 April.

**Good to know:** show a date to a user in the user's own zone. Otherwise the day can be one
too early or one too late.
