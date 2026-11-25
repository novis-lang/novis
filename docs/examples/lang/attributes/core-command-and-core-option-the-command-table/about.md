`#[Core\Command]` makes a static method a command for the command line, and `#[Core\Option]` makes
one of its parameters an option. The compiler collects every command in the program into one table
and checks it.

A command needs a `name:`, which is the word that selects it, and the name is unique across the
program. A parameter is a positional argument unless it carries `#[Core\Option]`, and a `bool` option
is a flag. Every option or positional parameter needs a type that text converts to: `string`, `int`,
`uint`, `decimal`, `bool`, an enum, a time value, or a class that implements `Parses`. A time value is
a `Core\Time\Date`, `TimeOfDay`, `Instant` or `DateTime`, written as ISO text such as `2026-03-14`.
A `DateTime` needs its time zone, as in `2026-03-14T09:00:00+01:00[Europe/Vienna]`. A word that does
not convert is a usage error. The method is `static` and returns `void` or the exit status as a `uint`.

`Core\Command::run()` reads the program's command line and calls the command that the first word
names. `Core\Command::help` returns the help page for one command, or the list of every command.
`Core\Attributes::get<Core\Command>` returns the `name` and `about` of one command, and
`Core\Attributes::get<Core\Option>` returns the fields of one option. The fourth example shows an
option that does not compile. The fifth example reads a date and a moment in time from the command
line.
