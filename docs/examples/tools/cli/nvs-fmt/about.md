`nvs fmt` rewrites Novis files in one fixed layout. A body is indented by four spaces. Each statement
and closing brace is on its own line.

Inside a line, the two tokens decide if there is one space or no space between them. `$a+$b` becomes
`$a + $b`. Several blank lines in a row become one.

The command has no style options and no line width. A line break that you wrote stays. If you break a
condition, a ternary or a chain of `+`, `&&` or another operator, each part gets its own line, with the
operator at the start. A `.` chain keeps your lines. A formatted file stays the same.

`nvs fmt app.nvs` formats one file in place. `nvs fmt src` formats every `.nvs` file in `src` and its
subfolders.

`--check` and `--diff` write nothing. `--check` prints the name of each file that would change, and
`--diff` prints the changes. The exit status is `1` when a file would change, and `0` otherwise. A file
with a syntax error is not changed, and the status is `2`.

`--stdin` formats standard input and prints the result. An editor uses it for a file that is not saved.
