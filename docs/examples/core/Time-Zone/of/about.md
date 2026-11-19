Finds a time zone by its name, such as `Europe/Berlin`, `America/New_York` or `Asia/Tokyo`.

The names come from the IANA time-zone database, which most operating systems use. A named zone
has the whole history of its region, including the days when summer time starts and ends. Novis
has no default zone, so you need one each time you turn an `Instant` into a date and a clock time.

A zone you know when you write the program does not need `Zone::of`. Write its name in quotes,
such as `$instant->in('Europe/Berlin')`. This name is checked while compiling, so a wrong name
does not compile.

Use `Zone::of` for text that arrives while the program runs, such as a name a user saved. A name
that is not in the database throws a `RuntimeError`. An offset such as `+02:00` is not a name, so
`Zone::of` throws for it too. Use `Zone::fixed` for an offset.
