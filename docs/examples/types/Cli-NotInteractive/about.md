The error a question raises when there is nobody there to answer it.

A command-line program that asks something — a name, a yes or no, a choice from a list — can be
started where no person is watching: a nightly job, a build step, a queue worker. Novis never blocks
there waiting for an answer that will never arrive. A question that was given a fallback answer takes
it and carries on; a question that was not raises this error straight away.

That makes the choice yours and makes it explicit. A program that should keep working unattended
names a fallback for every question it asks. A program that must not guess — anything that deletes,
sends or charges — catches this instead and stops, saying what it needed.

**Good to know:** a question is put to the terminal the program was started from, not to its standard
input, so a program reading piped data can still ask one.

**The examples below** show a question nobody answers, the same question given a fallback, and a
release script that refuses to guess.
