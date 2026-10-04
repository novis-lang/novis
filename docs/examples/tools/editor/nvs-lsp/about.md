`nvs lsp` is the language server that gives an editor its errors, completions and colours for Novis
code.

Your editor starts it. It reads requests on standard input and writes the results on standard
output. In a terminal it prints nothing and looks like it has stopped. It takes no arguments,
because the editor sends every setting when it connects.

The server reports the errors that `nvs check` reports. It shows the documentation of the name
under the cursor and finds where that name is declared. It completes methods, constants, enum
cases, keywords and variables, and it gives the outline of a file. It offers quick fixes for some
errors. One fix corrects the upper and lower case of a name, and another rewrites an old cast to
`as`. It can also convert a string to an html template that prints the same text. The server also
tells the editor which values are written to a `secret` variable, so the editor can hide them on
the screen.

**Good to know:** while a file has a syntax error, the server reports only that error. The errors
about names and types are shown again when the file parses.
