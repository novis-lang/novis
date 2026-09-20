Returns the number of bytes of memory your program is using right now. This replaces PHP's
`memory_get_usage`.

The number covers the values your program has made and still uses: its strings, arrays and objects.
It goes down again as soon as a value is no longer used, so a program that has finished with a large
file reads a small number again. You do not have to wait for the program to end.

Read it before a piece of work and again after it. The difference between the two readings is what
that work costs. Compare it with `Core\Budget::memoryLimit` to see how much room is left.

**Good to know:** this is the memory of your program alone, not of the whole server. For the whole
server, use `Core\Os::residentBytes`.
