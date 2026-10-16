Returns the Unix timestamp of this instant: the number of whole seconds since 1 January 1970 at
midnight UTC.

The result is an `int`. A moment before 1970 gives a negative number. The part of a second is
dropped, and the result is not rounded. Dropping always moves toward zero, so half a second before
1970 gives `0`, not `-1`.

Every instant has a Unix timestamp, so `toEpochSeconds` never throws an error. Use
`toEpochMillis` or `toEpochMicros` when you need the part of a second too.
