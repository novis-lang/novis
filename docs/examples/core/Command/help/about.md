Gives you the usage page of a command-line program, written from the commands the program declares.

You mark a method with `#[Core\Command]`, and its parameters with `#[Core\Option]` where they are
options. Novis reads those declarations while it compiles your program and builds a table from them.
`Core\Command::help` turns that table into a page: the command, what it does, its arguments and its
options, each with the type it was declared at. You never write usage text, so the page and the code
cannot disagree.

Pass a command name for that one command's page. Pass `null` for the program's own page, which lists
every command it has. A name the program declares no command for throws a `LogicError`.

**Good to know:** the result is a `Core\Cli\Text`, so you can print it with `echo`. The page already
ends with a newline.
