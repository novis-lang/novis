Two members show you what a value really is while you are working on a program. `Core\Debug::dump`
writes one rendering per argument to standard error. `Core\Debug::render` builds the same rendering
and returns it as text, so you can print it, put it in a log line, or compare it.

A rendering names the type of every value, the length of every string and the class of every object.
Control bytes such as a tab are made visible, a structure that is very deep or very long is cut
short, a value that points back at itself is marked, and a property declared `secret` is shown as
`[redacted]`.

`dump` never writes to standard output, and a captured buffer never swallows it. What your program
prints stays exactly what it printed, so a `dump` you forget to remove cannot change a response.

**The examples below** show a rendering printed, a `dump` beside a program's own output, and a
password that stays hidden.
