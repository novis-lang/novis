An option the attribute does not have is `E0454`, and the help line lists every option with its
type. An option of the wrong type is `E0401`, and an option given twice is `E0304`. A value that is
not known while compiling is `E0725`, as in any attribute. A missing option that the attribute needs
has an error of its own, such as `E0767` for a `#[Core\Command]` with no `name`.

A name under `Core\` that Novis does not read, such as `#[Core\Nope]`, is `E0726`. These names are
never written with `::`, so `#[Core\Json::Derive]` is `E0405`. An import cannot be renamed, so
`use Core\Command as Cmd;` is `E0212`.

Novis does not read an attribute without a name. `#[{ name: "deploy", about: "…" }]` is ordinary
metadata, even when its fields are the same as a `Core\Command`.

Your code can still read these attributes with `Core\Attributes`. For example,
`Core\Attributes::get<{ path: string }>(Home::show)` returns the `path` of a `#[Core\Route]`.

Each of these attributes belongs on one kind of declaration. A `#[Core\Command]` belongs on a
`static` method, and on a method that is not `static` it is `E0789`.
