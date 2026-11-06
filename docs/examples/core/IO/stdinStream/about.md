Reads a program's standard input in pieces, as bytes, for a `foreach` loop.

Each loop step gives the next piece of the input, up to 64 KiB. The loop ends when the input ends.
Only one piece is in memory at a time, so a very large input works too. The bytes are given as
they are, so input that is not UTF-8 text is read without an error.

Each piece is tainted, because it comes from outside the program. The input can be read only once.
A program started with no input gets no pieces. `Core\Hash::of` accepts the result directly, so one
line computes a checksum of the whole input.

**The examples below** count the bytes of the input, compute its checksum, and read input from an
older system that does not write UTF-8.
