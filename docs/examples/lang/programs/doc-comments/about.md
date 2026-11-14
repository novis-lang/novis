A doc comment is a note for whoever uses your code, and the toolchain reads it.

Write it with three slashes, `///`, on the lines directly above a class, a constant, a property, a
method or an enum case. The whole run of `///` lines is one comment. Your editor shows it to whoever
types that name, and `nvs doc` puts it on the page it builds.

Five tags may be written, each at the start of a line. `@see` names another class
or member, and `@example` names a file that shows the code in use. `@param $name` describes a
parameter, `@return` describes the return value, and `@throws Name` describes an error the method
throws. Each of these three needs a sentence after it. The compiler checks every tag against the
code, so a tag that names something that is not there stops the build. Any other `@tag` is an error.

**Good to know:** a tag never contains a type, because the signature already has it. A `///` run
with a blank line under it documents nothing, and that is an error. Four slashes or more is an
ordinary comment.
