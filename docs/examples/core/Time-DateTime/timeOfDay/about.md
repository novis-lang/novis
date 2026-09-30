Returns the time of day of a date and time, as a `Core\Time\TimeOfDay`.

The result has the hour, the minute, the second and the nanoseconds. It has no date and no time
zone. Use it to compare a value with a fixed time, for example to check whether a message arrived
during opening hours. You can also give the result to `withTime` to copy the time to another day.

**Good to know:** `timeOfDay` reads the clock in the time zone of the value. 12:00 in UTC on a
summer day is 14:00 in Berlin and 08:00 in New York. To get the time for a certain place, convert
the value to that time zone first.
