Reads a local date and time written as text, such as `2024-06-01 09:30`, and returns a `DateTime`
in the time zone you give.

The text is a date, `YYYY-MM-DD`, and can have a time after a `T` or one space. The time is
`HH:MM`, `HH:MM:SS` or `HH:MM:SS.f`. A date alone gives the start of that day. A local time that the
clocks skip or repeat is handled as in `Core\Time\DateTime::at`.

A text with an offset from UTC, such as `+02:00` or `Z`, throws a `ParseError`. For that text, use
`Core\Time\Instant::fromIso($text)->in($zone)`. A date that does not exist also throws a
`ParseError`.

Write a fixed time zone as a string, such as `'Europe/Berlin'`. For a zone name read while the
program runs, pass `Core\Time\Zone::of($name)`.

**The examples below** show a meeting time and a date alone, texts with an offset, and one local
time in several time zones.
