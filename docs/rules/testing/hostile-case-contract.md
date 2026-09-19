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
case. What cannot be caught — a memory limit, `exit` — is the file's last step, and the case says
that its ending is the attack; running to the last line is then what fails it.

Freezing the output instead is refused: every one of these programs is written to produce output
nobody can predict, and a suite whose expectations must be maintained is a suite that gets weakened
until it passes.
