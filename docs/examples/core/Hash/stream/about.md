Starts a digest that you feed piece by piece, for data that arrives in parts.

`Core\Hash::stream` returns a `Core\Hash\Stream` for the algorithm you choose. You give it the data
with `update`, one piece at a time, and you get the digest with `finish`. The result is the same
digest that `Core\Hash::of` returns for all the pieces joined together. How you cut the data into
pieces does not change the result.

**Good to know:** the stream keeps every piece until `finish`, so its memory grows with the data
you feed it. After `finish` the stream is closed. For the next digest, start a new stream.
