Most attributes are metadata. Novis stores what you attach and your program reads it back, and nothing
else happens. A small set of names is different. Novis owns those names, and when an attribute points
to one of them the compiler acts on it: it writes a JSON codec for the class, adds a row to the route
table, adds a row to the command table, or registers a test.

Novis decides by the declaration a name points to, not by the way the name is written. `#[Core\Command]`
and an imported `#[Command]` are one attribute. A type of your own called `Command` is a different
declaration, so it stays ordinary metadata however it is spelled. An import binds one whole name, so
`use Core\Json;` followed by `#[Json\Derive]` points at nothing and does not compile.

Each name Novis owns has a fixed list of options. An option it does not know, an option of the wrong
type, and an option given twice are each an error while your program compiles.

The examples show one attribute written both ways, an attribute of your own that shares a name with one
of these, and a class that carries both kinds at once.
