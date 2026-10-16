Builds a `bytes` value by repeating a whole buffer.

You give the buffer and the number of copies you want. The result is a new value holding those
copies one after another, in the order they were written. Zero copies return a value with nothing in
it. A buffer that is already empty stays empty, however many copies you ask for.

`Core\Bytes::fill` repeats a single byte. `Core\Bytes::repeat` repeats a buffer of any length, so one
copy can be several bytes long. A result larger than the memory the program may use throws an error,
and no value is built.

**Good to know:** this is how a program draws a line under a title, builds a block of a known
pattern, or makes a body of a fixed size to send in a test.
