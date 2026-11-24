Returns the number of days in the month of this date and time, from `28` to `31`.

February has `29` days in a leap year and `28` days in other years. The month is the month in the
value's own time zone. One moment can be 31 January in London and 1 February in Berlin, so the
result can depend on the time zone.

Use `daysInMonth` to find the last day of a month, for example for a bill that is due on the last
day. `with({ day: $dateTime->daysInMonth() })` gives that day.
