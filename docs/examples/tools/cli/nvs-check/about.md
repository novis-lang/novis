`nvs check <file>` finds the errors in a program and runs nothing.

It checks the file and every file that the program loads through `require` and `autoload`. When
there is no error, it prints `no errors` and the exit status is `0`. When there are errors, it
prints every one of them and the exit status is `1`. Nothing runs, so the command is safe to use
from an editor, a commit hook or a build server.

Each error has a code, a message, and the file, line and column. A code is `E` and four digits for
an error, and `W` for a warning. A code never changes, so you can search for it.

`--autoload-map` prints the `autoload` rules as the compiler resolved them, in place of
`no errors`. Use it to check an `autoload` declaration without a run.
