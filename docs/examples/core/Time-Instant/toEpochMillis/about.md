Returns the number of whole milliseconds since 1 January 1970 at midnight UTC. A millisecond is one
thousandth of a second.

The result is an exact `int`. A moment before 1970 gives a negative number. The
part of a millisecond is rounded down, to the earlier millisecond. So half a millisecond before 1970
gives `-1`.

JavaScript, many databases and many web APIs count time in milliseconds. Every instant has a
millisecond count that fits in an `int`, so `toEpochMillis` never throws an error.
