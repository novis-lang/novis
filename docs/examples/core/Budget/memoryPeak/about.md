Returns the largest amount of memory your program has used at any point, in bytes. This replaces
PHP's `memory_get_peak_usage`.

`Core\Budget::memoryHeld` gives you the memory your program is using right now, and that number goes
down again when a value is freed. The peak keeps the highest point instead. Work that is already
finished is still visible in it. A program that read a large file and kept a short summary of it has
a small figure for `memoryHeld` and a large one for the peak.

Read it at the end of a request to see how close that request came to `Core\Budget::memoryLimit`.
That is how you find the requests that are about to fail.

**Good to know:** the peak is never lower than what your program is using right now, and nothing
your program does can lower it.
