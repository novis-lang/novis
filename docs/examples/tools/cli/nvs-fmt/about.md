`nvs fmt` rewrites Novis files in one fixed layout. A body is indented by four spaces. Each statement
and each closing brace is on its own line. A plain string uses single quotes.

The command has no style options, so every project gets the same layout. It does not move a line break
that you wrote, and it has no line width. Running it on a formatted file changes nothing.

`nvs fmt app.nvs` formats one file in place. `nvs fmt src` formats every `.nvs` file in the folder
`src` and in its subfolders.

`--check` writes nothing. It prints the name of each file that would change. `--diff` also writes
nothing, and it prints the changes as a diff. The exit status is `0` when nothing would change and `1`
when a file would change. A build server runs `nvs fmt --check` and fails the build on status `1`.

A file with a syntax error is not changed. `nvs fmt` prints the error, and the exit status is `2`.

`--stdin` reads a file from standard input and prints the formatted text. An editor uses it to format
a file that is not saved yet.
