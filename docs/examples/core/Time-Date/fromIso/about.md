Reads a date written as text, such as `2024-03-01`, and returns a `Core\Time\Date`. The text must
follow ISO 8601, the international standard way to write a date.

The text must have the form `YYYY-MM-DD`: a four-digit year, a two-digit month and a two-digit day,
with a `-` between them. Other ISO 8601 forms are not allowed. These include `20240301`, a week date
such as `2024-W09-5`, and a day of the year such as `2024-061`.

A text that is not in this form throws a `ParseError`. A date that does not exist, such as
`2024-02-30`, throws one too. The message says which part of the text is wrong. A text written
directly in the code is checked when the program compiles, so `Date::fromIso('2024-02-30')` does not
compile. A text from a variable is checked when the program runs.

The same rules read a `Date` in a route path, a `#[Core\Query]` value, a command argument and JSON
that `Core\Json::decodeAs` reads.

**Good to know:** the year must be between -9999 and 9999, the same range `Core\Time\Date::at` takes.
A year before year 0 starts with `-`, such as `-0044`. Year 0 is written `0000`, and `-0000` throws
a `ParseError`.

**The examples below** show how to read a date, texts that are not valid dates, and how to check
whether an invoice is overdue.
