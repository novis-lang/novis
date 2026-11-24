Reads a date and time written as text, such as `2024-06-01 09:30`, and returns a `DateTime`.

The text is a date, `YYYY-MM-DD`, and can have a time after a `T` or one space. The time is
`HH:MM`, `HH:MM:SS` or `HH:MM:SS.f`. A date alone gives the start of that day. A local time that the
clocks skip or repeat is handled as in `Core\Time\DateTime::at`.

The text can end in an offset and a zone name in brackets, such as
`2024-06-01T09:30:00+02:00[Europe/Berlin]`. This is the text that `toIso` writes. For this text,
`fromIso` needs no second argument. A text with no zone name needs the time zone as the second
argument.

A text with an offset and no zone name, such as `+02:00` or `Z` alone, throws a `ParseError`. For
that text, use `Core\Time\Instant::fromIso($text)->in($zone)`. A date that does not exist also
throws a `ParseError`.

Write a fixed time zone as a string, such as `'Europe/Berlin'`. For a zone name read while the
program runs, pass `Core\Time\Zone::of($name)`.

**The examples below** show a meeting time and a date alone, texts with an offset, and one local
time in several time zones.
