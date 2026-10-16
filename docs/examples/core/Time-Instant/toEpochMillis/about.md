Returns the number of whole milliseconds since 1 January 1970 at midnight UTC. A millisecond is one
thousandth of a second.

The result is an `int`, so no digits are lost. A moment before 1970 gives a negative number. The
part of a millisecond is dropped, and the result is not rounded. Dropping always moves toward zero.

JavaScript, many databases and many web APIs count time in milliseconds. Every instant has a
millisecond count that fits in an `int`, so `toEpochMillis` never throws an error.
