Reads a date and time written as text, such as `2024-06-01T09:30:00+02:00`, and returns an
`Instant`. The text must follow ISO 8601, the international standard way to write a date and time.

The text needs a date, a time and an offset from UTC. The offset is `Z` for UTC, or a value such as
`+02:00`. Part of a second, such as `.25`, is kept. A text without an offset throws a `ParseError`.
For a time without an offset, use `Core\Time::parse` with a time zone. A date that does not exist
also throws a `ParseError`, and the message says which part of the text is wrong. So does a time
outside about the years -9999 to 9999.

**The examples below** show a time with an offset, texts that are not valid timestamps, and how long
a job ran from two times a web service returned.
