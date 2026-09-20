Builds a `bytes` value out of one byte repeated.

You give the number of bytes and the byte itself, written as a number from `0` to `255`. The result
is a new value of that size where every byte is the same. A length of `0` returns a value with
nothing in it.

A number above `255` does not fit in one byte, so `Core\Bytes::fill` throws an error. A length
larger than the memory the program may use also throws an error, and no value is built.

**Good to know:** this is how a program makes a block of zero bytes for a header it writes later, or
the padding that brings a record up to a fixed size.
