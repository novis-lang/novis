Asks a question at the terminal and returns the line the person typed.

A command line program often needs one thing from the person running it: a name, a path, a port
number. `Core\Cli::ask` writes the question and returns the answer, without the line ending. It
reads the terminal itself and not the standard input, so a program that is reading piped data can
still ask. A `validate` function of yours checks each answer, and an answer it turns down is asked
again.

Where there is no terminal, a nightly job for example, nobody can answer. Give a `default` and
that is what you get. Give none and the call throws `Core\Cli\NotInteractive` at once, so the
program never waits for an answer that cannot come.

**Good to know:** the answer is tainted. A person typed it, so anything can be in it. Check it
before you use it as a file path or in a database query.

**The examples below** ask one question, ask again until the answer is a number, and fill in the
settings a new project needs.
