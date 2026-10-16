Gives the words a program was started with, so the program can read its own command line.

When you start a program in a terminal you may write extra words after its name. Those words are
how a person tells the program what to do: a file to open, a name to look up, an option such as
`--verbose`. `Core\Cli::arguments` returns those words as a list of strings, in the order they were
written. A program started with no extra words gets an empty list. The name of the program itself
is never in the list, and a word that contains spaces stays one element.

**Good to know:** every word in the list is tainted. That means the text came from outside the
program, so anything can be in it. You may print such a word or compare it with text of your own.
Before you use one as a file path, a web address or part of a database query, check it against the
values you expect.

**The examples below** print the words, read an option from them, and choose a job with a fixed
list of allowed names.
