Runs a command-line program: it reads the words the program was started with, calls the command they
name, and gives you the status to end with.

You mark each command's method with `#[Core\Command]`. Novis builds a table of them while it
compiles your program, and `Core\Command::run` matches the command line against that table. The
first word chooses the command. The words after it fill that command's parameters, converted to the
types the parameters were declared at.

The result is the number the handler returned, or `0` for a handler declared `void`. A command line
this program cannot act on is a usage error: the usage page goes to standard error and the result is
`2`. Your program gives that number to `exit`.

**Good to know:** a command whose handler the program does not declare, or a parameter of a type no
word can be converted into, throws a `LogicError`. Those are mistakes in the program, not in what the
user typed.
