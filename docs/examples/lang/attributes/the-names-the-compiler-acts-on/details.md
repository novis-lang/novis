An option the attribute does not have is `E0454`, and the help line lists every option with its
type. An option of the wrong type is `E0401`, and an option given twice is `E0304`. A value that is
not known while compiling is `E0725`, as in any attribute. A missing option that the attribute needs
has an error of its own, such as `E0767` for a `#[Core\Command]` with no `name`.

A name under `Core\` that Novis does not declare, such as `#[Core\Nope]`, is `E0303`. These names are
never written with `::`, so `#[Core\Json::Derive]` is `E0405`. An import cannot be renamed, so
`use Core\Command as Cmd;` is `E0212`.

An attribute of your own with the same fields as `Core\Command` is still your own attribute. Novis
does not read it.

`Core\Attributes` can read these attributes too. `Core\Attributes::get<Core\Route>(Home::show)`
returns the fields written in the route attribute, such as its `path`. Some also have a reader of
their own, such as `Core\Command::help` for the command table.

Each of these attributes belongs on one kind of declaration. A `#[Core\Command]` belongs on a
`static` method, and on a method that is not `static` it is `E0789`.
