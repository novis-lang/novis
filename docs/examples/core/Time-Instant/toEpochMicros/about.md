Returns the number of whole microseconds since 1 January 1970 at midnight UTC. A microsecond is one
millionth of a second.

The result is an exact `int`. A moment before 1970 gives a negative number. The
part of a microsecond is rounded down, to the earlier microsecond. So half a microsecond before
1970 gives `-1`.

Log tools and some databases store time in microseconds. Every instant has a microsecond count that
fits in an `int`, so `toEpochMicros` never throws an error.
