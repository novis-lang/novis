Writes this number as text in the base you choose, and returns that text as a `string`.

The base is a whole number from 2 to 36. The digits above 9 are the lower case letters `a` to `z`,
so base 16 writes 255 as `ff`. A negative number gets one `-` in front of it. Without the `radix`
option the base is 10, which gives the same text as `toString`.

`Core\BigInt::parse` reads back what `format` writes, as long as you give it the same base. You can
use that pair to keep a very large number in a file, a database column or a web address, and read it
back later with nothing lost.

**Good to know:** a base below 2 or above 36 throws a `LogicError`. There are ten digits and 26
letters, so 36 is the largest base this class can write.
