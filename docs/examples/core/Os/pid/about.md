Returns the number the operating system gives this program's process.

The number does not change while the process runs. It belongs to the whole process, not to one
request. A server process that handles many requests returns the same number for all of them. Two
processes that run at the same time on one machine never have the same number.

Use it to tell processes apart: in a log line, in the name of a worker, or in the name of a file
that only this process writes. The number is different on every run, so do not write it into a
test.

The examples show how to read the number, that it stays the same, and how to put it in a file name.
