Returns the date and time as ISO 8601 text, with its offset from UTC and its time zone, such as
`2026-01-05T08:30:00+01:00[Europe/Vienna]`.

The text has the local date and time, then the offset, then the zone name in brackets. It always
has seconds. It has a fraction of a second only when the fraction is not zero, such as
`09:30:15.25`.

`Core\Time\DateTime::fromIso` reads this text back, with no second argument. The result is the
same moment in the same time zone. When the clocks go back, a local time happens twice. The offset
in the text shows which of the two times it is.

Use `toIso` to save a `DateTime` in a file or a database column, or to send it to another program.
For text a person reads, use `format`.
