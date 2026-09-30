Counts the whole units from one date and time to another, such as years, months or days.

You give the other value and a unit from `Core\Unit`. The result is a whole number, and a part of a
unit is not counted. From a birthday on 15 May 1990 to 14 May 2024 is 33 years. One day later it is
34. The result is negative when the other value is earlier.

The count uses the calendar in the time zone of the value you call `difference` on. The other value
can be in a different time zone.

**Good to know:** a day is not always 24 hours. In Berlin the clocks move forward on 31 March 2024.
From 12:00 on 30 March to 12:00 on 31 March is 1 day, and it is 23 hours.

If you come from PHP, `difference` replaces `date_diff`.
