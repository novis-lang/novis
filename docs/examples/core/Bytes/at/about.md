Returns one byte of a `bytes` value as a number from 0 to 255.

The position counts from the start, so `0` is the first byte. A negative position counts from the
end, so `-1` is the last byte. If the position names a byte that is not there, this throws an error.
Catch `Throwable` around it when the position comes from outside your program.

**Good to know:** the result is a number, not a `bytes` value of one byte. A number is what you need
to compare a byte with a known value, such as the `0x89` that every PNG file starts with.
