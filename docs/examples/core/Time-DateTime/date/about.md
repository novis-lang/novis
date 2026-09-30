Returns the calendar day of a date and time, as a `Core\Time\Date`.

The result has the year, the month and the day. It has no time of day and no time zone. Use it
when only the day matters, for example to count orders for each day or to compare a value with a
due date.

**Good to know:** `date` reads the day in the time zone of the value. One moment can be a different
day in another place. 20:00 on 31 December in New York is already 1 January in Tokyo. To get the
day for a certain place, convert the value to that time zone first and then call `date`.
