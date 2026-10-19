`#[Core\Command]` has two fields, `name` and `about`. A command with no `name` is `E0767`, and a
`name` that is not a string is `E0401`. `about` is optional. `#[Core\Option]` has three fields,
`short`, `long` and `about`, and each is optional. Any other field is `E0454`. You can write
`#[Command(...)]` and `#[Option(...)]` after `use Core\Command;` and `use Core\Option;`.

Two commands with one name are `E0768`. This is true when they are in two classes, and when one method
has two `#[Core\Command]` lines with the same name. Two lines with different names on one method give
one command two names, and `Core\Attributes::all` returns both lines.

A method that is not `static` is `E0789`. A return type other than `void` or `uint` is `E0789` too,
and that includes `int`. `#[Core\Option]` on a parameter of a method with no `#[Core\Command]` is
`E0766`.

Each option has a long spelling. It is `--` and the `long` value, or `--` and the parameter's name when
there is no `long`. With `short: "v"`, the option also has the spelling `-v`. Two options of one command
with the same spelling are `E0745`. So `long: "name"` on one parameter and a second parameter called
`$name` are `E0745`. `short: "v"` and `short: "V"` are two different spellings.

An option of a type that text does not convert to is `E0746`. A `?string` option is `E0746` too. A
positional parameter is not checked this way. A positional `float` compiles, and `Core\Command::run`
throws `LogicError` when it reaches that command.

`Core\Command::run` fills the parameters from the words after the command's name. An option may come
before, between or after the positional words. An option with a value is written `--retries 7`. The
form `--retries=7` is not an option. A `bool` option is `true` when it is written and `false` when it
is not, even with the default value `true`. An option with a default value may be left out. An enum
parameter takes the name of a case, written exactly, so `Fast` works and `fast` does not.
`--help` is not an option of its own, so `deploy --help` is a usage error.
