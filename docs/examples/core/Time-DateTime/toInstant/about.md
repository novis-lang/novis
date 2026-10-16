Returns the exact moment of a date and time, as a `Core\Time\Instant`.

A date and time is a local time in one time zone, such as 14:30 in Vienna. An `Instant` is a point
in time that is the same everywhere, and it has no time zone. `toInstant` gives that point. 14:30 in
Vienna on 1 July 2024 is 12:30 in UTC.

Use it to compare values from different time zones. 09:00 in London and 18:00 in Tokyo on 15
January 2024 are the same moment, so their instants are equal. Use it also to measure the real time
between two values, such as how long a flight takes.

**In plain words:** the local time is what a clock on the wall shows in one city. The instant is
the moment itself, and it is the same moment in every city.
