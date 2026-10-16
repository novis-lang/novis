Reads a date and time from text, using a pattern that says how the text is written, and returns a
`DateTime` in the time zone you give.

The pattern uses letters for each part: `yyyy` is the year, `MM` the month, `dd` the day, `HH` the
hour and `mm` the minute. Other characters, such as `-` or `.`, must appear in the text exactly as
written. A pattern without a time gives midnight. A text that does not match the pattern throws a `ParseError`, and the message says
where the text stopped matching. So does a time outside about the years -9999 to 9999. A pattern that is not valid, or that names a time zone, throws a
`LogicError`.

**The examples below** show a date in a European format, texts that do not match, and times from a
file converted to UTC.
