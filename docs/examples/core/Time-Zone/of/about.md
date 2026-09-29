Finds a time zone by its name, such as `Europe/Berlin`, `America/New_York` or `Asia/Tokyo`.

The names come from the IANA time-zone database, which most operating systems and programming
languages use. A named zone has the whole history of its region: the days when summer time starts
and ends, and every change to its offset in the past. You need a zone each time you turn an
`Instant` into a date and a clock time, because Novis has no default zone.

A name that is not in the database throws a `RuntimeError`. A misspelled name such as
`Europe/Berln` throws too, so you see the mistake the first time the line runs. An offset such as
`+02:00` is not a name, and `Zone::of` throws for it. Use `Zone::fixed` for an offset.
