Reports whether one of the three standard streams is a terminal.

A program's output goes to a terminal, or to a file, or to another program. `Core\Cli::isTty` tells
you which. Ask about standard output before you print colours or draw a progress line: a file keeps
those characters, and the person who reads the file later sees them as noise. Ask about standard
input before you prompt, because nobody is there to answer when the input comes from a file.

The answer is resolved once, when the program starts, so two calls in one run always agree.

**Good to know:** each of the three streams can answer differently. Standard output is a terminal
while standard input is a pipe whenever somebody runs `cat orders.txt | your-program`.

**The examples below** ask about each stream, choose what to print when there is no terminal, and
skip a question nobody can answer.
