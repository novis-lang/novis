Writes what a value really is to standard error, so you can look at it while a program runs.

You can pass any number of values in one call. For each one, `Core\Debug::dump` writes a line that
names the type, the length of every string and the class of every object. A control character such
as a tab is made visible, a very deep or very long structure is cut short, a value that points back
at itself is marked, and a property declared `secret` is shown as `[redacted]`.

A dump never goes to standard output, and `Core\Out::capture` never swallows one. What your program
prints stays exactly what it printed, so a dump you forget to remove cannot change a response or
break another program that reads your output. Inside a web request a dump becomes a log line at the
`debug` level instead.

**The examples below** show a dump of the settings a program reads, a dump inside a loop, and a dump
beside output that another program reads.
