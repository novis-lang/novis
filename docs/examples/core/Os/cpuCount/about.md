Returns the number of CPU cores your program may use. The number is always 1 or more.

This is often smaller than the number of cores in the machine. A server can limit a program to some
of its cores, and a container can too. A program in a container with 2 cores on a machine with 96
cores reads 2. The web server uses the same number to decide how many workers it starts.

Use it when your program decides how many pieces of work to run at the same time. The number is
different on every machine, so do not write it into a file or a test.

The examples show how to read the number, how to give jobs out to the cores in turn, and how to choose
the size of a pool of workers.
