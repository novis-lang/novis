`#[Core\Command]` marks a static method as a command for the command line, and `#[Core\Option]` marks
one of its parameters as an option. The compiler collects every command in the program into one table
and checks it.

A command needs a `name:`, which is the word that selects it, and the name is unique across the
program. A parameter is a positional argument unless it carries `#[Core\Option]`, and a `bool` option
is a flag. Every option or positional parameter needs a type that text converts to: `string`, `int`,
`uint`, `decimal`, `bool`, an enum, or a class that implements `Parses`. The method is `static` and
returns `void` or the exit status as a `uint`.

`Core\Command::run()` reads the program's command line and calls the command that the first word
names. You can also read the table yourself with `Core\Attributes::get` and
`Core\Program::implementing`, as the examples below do. The last example shows an option that does
not compile.
