`nvs lsp` is the language server that gives an editor its errors, completions and colours for Novis
code.

Your editor starts it, and you do not run it yourself. It reads requests on standard input and
writes the results on standard output. In a terminal it prints nothing and looks like it has
stopped. It takes no arguments, because the editor sends every setting when it connects.

The server works the same way for every editor. It reports the errors that `nvs check` reports. It
shows the documentation of the name under the cursor and finds where that name is declared. It
completes methods, constants, enum cases, keywords and variables, and it gives the outline of a
file. It offers two quick fixes. One corrects the upper and lower case of a name. The other rewrites
an old cast to `as`. The server also tells the editor which values are written to a `secret`
variable, so the editor can hide them on the screen.

**Good to know:** while a file has a syntax error, the server reports only that error. The errors
about names and types are shown again when the file parses.
