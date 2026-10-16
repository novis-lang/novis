`#[Core\Command]` marks a static method as a command for the command line, and `#[Core\Option]` marks
one of its parameters as an option. The compiler collects every command in the program into one table
and checks it.

A command needs a `name:`, which is the word that selects it, and the name is unique across the
program. A parameter is a positional argument unless it carries `#[Core\Option]`, and a `bool` option
is a flag. Every option or positional parameter needs a type that text converts to: `string`, `int`,
`uint`, `decimal`, `bool`, an enum, or a class that implements `Parses`. The method is `static` and
returns `void` or the exit status as a `uint`.

**Good to know:** nothing runs a command for you in this build. There is no `Core\Command::run` yet.
What you have is the checked table, which you read with `Core\Attributes::get` and
`Core\Program::implementing`.
