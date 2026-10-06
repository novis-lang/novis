Returns the largest amount of memory this program may use, in bytes.

The person who runs the server sets this limit, and your program cannot change it. A program that
goes over the limit is stopped with an error. Read this number together with
`Core\Budget::memoryHeld` to see how much room is left, or with `Core\Budget::memoryPeak` to see how
close the program came.

**Good to know:** the result is `0` when this program runs with no limit at all. Test for `0` first.
Without that test, a program that may use as much memory as it likes looks like a program that is
already over its limit.
