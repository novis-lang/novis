Reads a date and time written as text, such as `2024-06-01T09:30:00+02:00`, and returns an
`Instant`, one exact moment in time. The text must follow ISO 8601, the international standard way
to write a date and time.

The text needs a date, `T` or one space, a time and an offset from UTC. The offset is `Z` for UTC,
or a value such as `+02:00`. Part of a second, such as `.25`, is kept. For a time without an
offset, use `Core\Time\DateTime::fromIso` with a time zone.

These texts throw a `ParseError`:

- a text without an offset
- a lower-case `t` or `z`
- a `,` before the part of a second
- the second `60`
- an offset without a colon, such as `+0100`
- a year that does not have four digits, such as `-009999`
- a date that does not exist, such as `2024-02-30`
- a time before `-9999-01-02T01:59:59Z` or after `9999-12-30T22:00:00Z`

The error message names the wrong part. A text written in the code is checked when the program
compiles.

**The examples below** show a time with an offset and texts that throw. The last one shows how
long a job ran.
