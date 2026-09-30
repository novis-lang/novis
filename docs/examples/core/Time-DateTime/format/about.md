Converts a date and time to text, in the layout that a pattern describes.

The pattern is a string of letters, and each group of letters is one part of the value. `yyyy` is
the year, `MM` is the month, `dd` is the day, `HH` is the hour, `mm` is the minute and `ss` is the
second. `EEEE` is the name of the weekday and `MMMM` is the name of the month, both in English.
`XXX` is the offset from UTC, and `VV` is the name of the time zone. Text inside single quotes is
copied to the result unchanged.

The result shows the value in its own time zone. To print the same moment for another place,
convert the value to that time zone first.

**Good to know:** a pattern with a mistake does not compile when you write it directly in the call.
A pattern that your program builds while it runs throws a `LogicError`. If you come from PHP,
`format` replaces `date` and `strftime`.
