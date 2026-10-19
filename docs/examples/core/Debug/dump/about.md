Writes what a value really is to standard error, so you can look at it while a program runs.

You can pass any number of values in one call. For each one, `Core\Debug::dump` writes a line that
names the type, the length of every string and the class of every object. A control character such
as a tab is made visible, and a very deep or very long structure is cut short. A value that points
back at itself is marked, and a property declared `secret` is shown as `[redacted]`.

On a terminal that shows colour, each type has its own colour. `NO_COLOR` and `CLICOLOR_FORCE`
change this the same way they change `Core\Cli::colorDepth()`.

A dump never goes to standard output, and `Core\Out::capture` never captures one. So a dump you
forget to remove cannot change a response. Inside a web request, a dump is a log line at the
`debug` level instead.

**The examples below** show a dump of the settings a program reads, a dump inside a loop, and a dump
beside output that another program reads.
