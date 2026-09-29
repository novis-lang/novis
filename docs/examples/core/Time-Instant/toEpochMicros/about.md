Returns the number of whole microseconds since 1 January 1970 at midnight UTC. A microsecond is one
millionth of a second.

The result is an `int`, so no digits are lost. A moment before 1970 gives a negative number. The
part of a microsecond is dropped, and the result is not rounded. Dropping always moves toward zero.

Log tools and some databases store time in microseconds. Every instant has a microsecond count that
fits in an `int`, so `toEpochMicros` never throws an error.
