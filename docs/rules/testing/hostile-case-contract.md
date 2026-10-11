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

The step is the number its step comment carries (`// 4.` or `// Step 4:`), and a file with one
attack is step `1`. The `step <N>` line is how the runner knows the program reached the step: a
`FATAL` prints only its message, with no class and no frames, because no backtrace is taken on the
path that stops a program at a limit, and `exit` prints nothing. One rule for every ending is simpler
for an author than one for throws and another for the rest. Where the error report does name a line,
the runner maps the outermost frame in the attack file to the step comment above it, and a failure
names the declared and the reached together: `stopped at step 2 with RuntimeError, declares step 4
with FATAL`. An attack that stops before its last step is fixed so the step runs; moving the marker
to the step it stops in is softening the case.

Freezing the output instead is refused: every one of these programs is written to produce output
nobody can predict, and a suite whose expectations must be maintained is a suite that gets weakened
until it passes.

**Declined:**
- **The error report's line alone.** It finds the step for a throw, and a throw is the ending an
  attack should not have. A `FATAL` has no frames, so the endings that matter most would go
  unchecked.
- **A class for each limit** (`MemoryLimitError`). No such class exists, and none should exist only
  so that a marker can name it; giving a `FATAL` a class would change what every operator reads for
  the sake of a test runner.
