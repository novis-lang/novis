Builds a date and time from three parts: a `Date`, a `TimeOfDay` and a time zone.

The result is a `DateTime`, which is always in a zone, so the same call gives the same moment on
every server. `TimeOfDay::at(0, 0)` is midnight. When every part is a fixed value, you can also write
`DateTime::fromIso('2024-03-05 09:30', 'Europe/Berlin')`.

A date that does not exist, such as 30 February, throws a `RuntimeError` in `Date::at`. A clock
time the zone skips is different. When clocks jump from 02:00 to 03:00, the time 02:30 moves
forward to 03:30, and there is no error.

**The examples below** build a date with and without a time, show what happens to a date or time
that does not exist, and show one meeting time in the zones of the people who join it.
