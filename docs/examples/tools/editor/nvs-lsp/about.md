`nvs lsp` is the language server that gives an editor its errors, completions and colours for Novis
code.

Your editor starts it. It reads requests on standard input and writes the results on standard
output. In a terminal it prints nothing and looks like it has stopped. It takes no arguments.

The server reports the errors that `nvs check` reports. It shows the documentation of the name
under the cursor and finds where that name is declared. It completes methods, constants, enum
cases, keywords and variables, and it gives the outline of a file. In a doc comment, you can click
each `@see`, `@example` and `@throws` target to open the file or the declaration it names.

It offers quick fixes (edits that repair an error) for some errors. One fix rewrites `(int)$x` to `$x as int`. The server also tells the editor
which values are `secret`, so the editor can hide them.

It also renames, moves, inlines and deletes code. If the new code would not compile, it changes
nothing.

**Good to know:** while a file has a syntax error, the server reports only that error. The errors
about names and types are shown again when the file parses.
