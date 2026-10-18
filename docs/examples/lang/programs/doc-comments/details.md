The body of a doc comment is Markdown. `nvs meta --json` includes it, `nvs doc` shows it on a page,
and the editor shows it when you hover over the name.

`@see` and `@example` are tags only at the start of a line. `@see` must name a class or member that
exists, or the program does not compile (`E0323`). `@example` must name a file under `examples/` or
`tests/` (`E0325`). Any other tag, such as `@param` or `@return`, does not compile (`E0128`).

A `///` comment must be directly above a class, a constant, a property, a method or an enum case. If it
is above a statement, above a blank line or at the end of the file, the program does not compile
(`E0127`). Write `//` for a note to the reader.

`/** … */` is an ordinary comment. Directly above a declaration it documents nothing, and the compiler
prints the warning `W1011`. A line of four or more slashes is an ordinary comment too.
