Names which of a program's three standard streams a call is about.

Every command-line program starts with three connections to the world outside it: standard input,
which carries what somebody types or what an earlier program in a pipeline produced; standard output,
which carries the answer; and standard error, which carries the notes, warnings and progress a person
reads while the program runs. `Core\Cli\Stream` has one case for each — `In`, `Out` and `Err` — so a
call that is about one stream names it, instead of there being three members or a piece of text to
misspell.

**In plain words:** output is the answer, error is the running commentary. Keeping them apart is what
makes `myprogram > results.txt` leave a file holding only the answer while the warnings still reach
the screen.

**Good to know:** input has a case because asking *is this a terminal* is a fair question about all
three streams. Writing to it is not, so `Core\Cli::write` throws rather than quietly doing nothing.
