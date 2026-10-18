The status is one byte. A number outside 0 to 255 keeps only its lowest eight bits, so `exit(300)`
ends with status `44` and `exit(-1)` ends with status `255`. This is the same on every system.

`exit("bye")` prints `bye` to standard output and adds no newline. Put `"\n"` at the end of the text
if you want one.

`die` does not compile (`E0228`). The message names `exit`, which takes the same argument.

A thrown error that nothing catches ends the program with status `1`. The error and the lines it
passed through are printed to standard error.
