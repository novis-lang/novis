A doc comment is a note for whoever uses your code. The compiler and the editor read it.

Write it with three slashes, `///`, directly above a class, a constant, a property, a method or an
enum case. Your editor shows it to whoever types that name, and `nvs doc` puts it on the page it
builds.

A doc comment can contain five tags, each at the start of a line. `@see` names a class, a method, a
property or a constant. It can also name a file in your project, such as `docs/billing.md#refunds`.
`@example` names a file that shows the code in use. `@param $name`, `@return` and `@throws Name`
describe a parameter, the return value and an error the method throws. Each needs a sentence after
it. The compiler checks every tag, so a tag that names something that does not exist stops the
build. Any other `@tag` is an error.

**Good to know:** a tag never contains a type, because the declaration already has it. A blank line
between a `///` comment and its declaration is an error. Four or more slashes start an ordinary
comment.
