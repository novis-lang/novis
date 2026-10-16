Returns the time zone of the computer the program runs on.

Novis has no default time zone. Each conversion between an `Instant` and a date with a clock time
needs a zone, and you write that zone in the call. `Zone::system` is how a program uses the zone of
the computer: you call it once and pass the result to each call that needs a zone.

The result has the name of the zone when the computer has one, such as `Europe/Berlin`. Some
computers have only an offset and no name. Then the result is a fixed zone at that offset, such as
`+02:00`. `Zone::system` never throws an error.

The same program can give different results on different computers. A server often runs in UTC,
and a laptop runs in the zone of its user. The examples do not print the local time for that reason.
