Builds an `Instant` from a Unix timestamp. A Unix timestamp is the number of seconds since
1 January 1970 at midnight in UTC. It replaces PHP's `DateTime::setTimestamp`.

A negative number of seconds is a time before 1970. The `nanos` option adds nanoseconds after the
second. So `-1` with `{nanos: 500000000}` is half a second after `-1`, not before it.

The seconds must be between `-377705023201` and `253402207200`. That is the time from the start of
year -9999 to the end of year 9999. A number outside that range throws a `RuntimeError`, and so
does a `nanos` value of one second or more.

**The examples below** show a timestamp from a database as a local time, a time before 1970, and a
check of whether a login token has expired.
