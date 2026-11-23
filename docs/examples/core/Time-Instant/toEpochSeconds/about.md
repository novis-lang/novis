Returns the Unix timestamp of this instant: the number of whole seconds since 1 January 1970 at
midnight UTC.

The result is an `int`. A moment before 1970 gives a negative number. The part of a second is
rounded down, to the earlier second. So half a second after 1970 gives `0`, and half a second
before 1970 gives `-1`. To get the same instant back, pass this number and the nanoseconds that
were dropped to `Core\Time\Instant::fromEpoch`.

Every instant has a Unix timestamp, so `toEpochSeconds` never throws an error. Use
`toEpochMillis` or `toEpochMicros` when you need the part of a second too.
