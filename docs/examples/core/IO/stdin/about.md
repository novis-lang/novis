Reads all of the text that was sent to a program's standard input.

Standard input is the text a program receives when it is started. In a terminal you send a file to
it with `<`, as in `nvs run report.nvs < orders.csv`, or with a pipe from another program.

`Core\IO::stdin` reads until the input ends and returns everything as one string. A program started
with no input gets the empty string. The input can be read only once, so a second call returns the
empty string too. In a terminal with no file or pipe, the call waits until the person there ends the
input with Ctrl-D (Ctrl-Z and Enter on Windows).

The result is tainted, because the text comes from outside the program. When the input is not valid
UTF-8, `stdin` throws a `RuntimeError`. It needs no capability.

**The examples below** count the lines of the input, show what a program with no input reads, and add
up a stock list.
