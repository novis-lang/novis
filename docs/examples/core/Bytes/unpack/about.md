Reads a buffer of bytes back into numbers and text, following the same format
that `Core\Bytes::pack` writes.

The buffer comes first and the format second. Each letter in the format describes one field, and the
result is a list with one entry per field, in the order the format names them. The list is
positional, so the first field is at position 0. A number code gives a number, and a text code gives
a `bytes` value with its own padding taken off.

The format must describe the whole buffer. Bytes left over after the last field throw an error, and
so does a field that reaches past the end of the buffer. To read a header out of a longer buffer,
cut the header off first with `Core\Bytes::slice`.

**Good to know:** the two members share one format. Reading a buffer with the format that wrote it
gives the same values back, field for field.

**The examples below** read number fields first, then text fields and their padding, then a message
that arrived over a socket.
