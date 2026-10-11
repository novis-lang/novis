`nvs code` answers questions about the project you are in: where a name is declared, where it is
used, which classes implement an interface, and which type a variable has. It reads the project the
way the editor does, and it changes nothing.

`nvs code refs 'Shop\Cart::addItem'` prints every place the method is written, the declaration
first. `nvs code impls 'Shop\Priced'` prints the classes that implement the interface. `nvs code
show` prints a name's signature and its doc comment. `nvs code outline <file>` lists what a file
declares, and `nvs code types <file>` prints the type of each `var` variable.

Each result line starts with `path:line:column`. The line and the column start at 1. With `--json`,
each command prints one JSON document for a tool to read.

**Good to know:** write the name with its namespace, in single quotes. Without the quotes, the shell
removes the backslashes and the name matches nothing. When a name is not found, the command exits
with an error and prints the nearest names.
