Returns the name of the machine your program runs on.

The name is the one the machine has for itself. Reading it does not use the network, so it is quick
and works on a machine with no network at all.

Use it when many servers do the same work and you need to know which one did something. Put it at
the start of a log line, or in the name of a worker that takes jobs from a queue. The name is
different on every machine, so do not write it into a test.

The examples show how to read the name, how to add it to a log line, and how to give each worker a
name that no other worker has.
