`#[Core\Command]` makes a static method a command for the command line, and `#[Core\Option]` makes
one of its parameters an option. The compiler collects every command into one table and checks it.

A command needs a `name:`, the word that selects it, and the name is unique in the program. A
parameter is a positional argument unless it has `#[Core\Option]`, and a `bool` option is a flag.
Each parameter needs a type that text converts to: `string`, `int`, `uint`, `decimal`, `bool`, an
enum, a time value, or a class that implements `Parses`. A time value is written as ISO text, such as
`2026-03-14`. A word that does not convert is a usage error. The method returns `void` or the exit
status as a `uint`.

`Core\Command::run()` calls the command that the first word names. `Core\Command::help` returns the
help page of one command, or the list of all commands. `Core\Attributes::get` reads the fields of a
command or an option.

The fourth example shows an option that does not compile. The fifth reads a date and a moment in
time from the command line.
