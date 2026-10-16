Returns the most memory this program's process has used so far, in bytes.

The number is the highest point, not the memory in use right now. It never gets smaller, even
when the program frees memory. It covers the whole process. A server process adds up every request
it has handled, so use `Core\Budget` for the memory of one request.

Use it to watch a long-running process: put it on a metrics page, or restart a worker when the
number gets too large. It is different on every machine, so do not write it into a test.

The examples show how to read the number, that it never gets smaller, and how to put it on a
metrics page.
