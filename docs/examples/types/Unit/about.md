The calendar step a date moves by: a nanosecond, a microsecond, a millisecond, a second, a minute, an
hour, a day, a week, a month, a quarter or a year.

You hand one of these to every member that moves a date forward or back, trims it to the start or the
end of something, or counts the distance between two dates. The cases are listed smallest first, so
comparing two of them reads the way it looks, and a week begins on Monday.

**Good to know:** a calendar step is not a fixed amount of time. One month after 31 January is the
last day of February, because the day number is clamped to the month it lands in, and in a zone that
puts its clocks forward a day is 23 hours rather than 24. Where you want a span that never changes
length — a timeout, a cache lifetime — reach for a duration instead.
