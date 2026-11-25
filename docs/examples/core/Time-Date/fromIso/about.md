Reads a date written as text, such as `2024-03-01`, and returns a `Core\Time\Date`. The text follows
ISO 8601, the international standard way to write a date.

The text must have the form `YYYY-MM-DD`. Other ISO 8601 forms are not allowed, such as `20240301`,
the week date `2024-W09-5` or the day of the year `2024-061`.

Text in another form throws a `ParseError`. So does a date that does not exist, such as
`2024-02-30`. The message says which part is wrong. Text written directly in the code is checked at
compile time, so `Date::fromIso('2024-02-30')` does not compile.

The same rules read a `Date` in a route path, a `#[Core\Query]` value, a command argument and JSON
that `Core\Json::decodeAs` reads.

**Good to know:** the year must be between -9999 and 9999. A year before year 0 starts with `-`,
such as `-0044`. Year 0 is written `0000`, and `-0000` throws a `ParseError`.

**The examples below** read a date, show texts that are not valid dates, and check whether an
invoice is overdue.
