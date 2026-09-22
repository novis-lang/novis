A doc comment is a note for whoever uses your code, and the toolchain reads it.

Write it with three slashes, `///`, on the lines directly above a class, a constant, a property, a
method or an enum case. The whole run of `///` lines is one comment. Your editor shows it to whoever
types that name, and `nvs doc` puts it on the page it builds.

Two tags may be written, each on its own line at the end of the comment. `@see` names another class
or member, and `@example` names a file that shows the code in use. Both are checked when the program
compiles, so a link that goes nowhere stops the build. Any other `@tag` is an error. Explain a
parameter in a sentence instead: its type is already in the signature.

**Good to know:** a `///` run with a blank line under it documents nothing, and that is an error.
Four slashes or more is an ordinary comment, so a divider line of slashes is not documentation.
