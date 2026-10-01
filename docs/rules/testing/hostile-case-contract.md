A hostile case is judged by a contract, never by frozen output. Its assertion is that nothing came
apart: a program that throws, one a limit stops, one that runs out of memory and says so, and one
that simply works are all passes. A panic, an abort, a hang past its timeout, a crash-shaped exit
status and a definite leak under valgrind are not.

The one failure that would otherwise look like a pass is a **compile diagnostic**, and it is checked
for by name — an attack that does not compile was never delivered, and a typo would otherwise
survive every sweep for the rest of this repository's life. Where the refusal *is* the assertion — a
sink handed a tainted value, a capability used without being granted — the case says so, and
compiling cleanly is then what fails it.

A program that **ends before its last line** is the same failure one step later: a case is several
attacks in one file, and an uncaught throw or a limit in the second of them delivers none of the
ones behind it. So every step that can be caught is caught, and a non-zero exit status fails the
case. What cannot be caught — a limit, `exit` — is the file's last step, and the case names that
step and its ending: `// hostile: ends-early <step> <ending>`. The ending is the class the error
report names for an uncaught throw, `FATAL` for a limit and `exit` for an `exit` with a non-zero
status, and the step prints `step <N>` on a line of its own as its first statement. The case passes
only when that line was printed and the program then stopped with the ending it names. Stopping
before the step, stopping with another ending, running to the last line, and a marker with no step,
no ending or a step that is not the last all fail it.

Freezing the output instead is refused: every one of these programs is written to produce output
nobody can predict, and a suite whose expectations must be maintained is a suite that gets weakened
until it passes.
