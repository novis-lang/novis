Converts a date to text, in the layout that a pattern describes.

The pattern is a string of letters, and each group of letters is one part of the date. `yyyy` is
the year, `MM` is the month with two digits and `dd` is the day with two digits. `MMMM` is the name
of the month and `EEEE` is the name of the weekday. Text inside single quotes is copied to the
result unchanged.

A date has no time of day and no time zone. A pattern with a letter for a time, such as `HH`, throws
a `LogicError`. A pattern that is not written correctly throws a `LogicError` too.

**Good to know:** `Core\Time\DateTime::format` reads the same letters, and it also has the letters
for a time of day.
