`nvs code` answers questions about the project you are in: where a name is declared, where it is
used, which classes implement an interface, and which type a variable has. It reads the project the
way the editor does. Only `rename`, `move` and `order` change files.

`nvs code refs 'Shop\Cart::addItem'` prints every place the method is written, the declaration
first. `nvs code impls 'Shop\Priced'` prints the classes that implement the interface. `nvs code
show` prints a name's signature and its doc comment.

`nvs code order` puts the members of each class in the standard order: constants, then properties,
then methods. `nvs code order --check` changes nothing. It prints each class that is not in order
and exits with an error, so a CI job can run it.

Each result line starts with `path:line:column`. With `--json`, each command prints one JSON
document for a tool to read.

**Good to know:** write the name with its namespace, in single quotes. Without the quotes, the shell
removes the backslashes and the name matches nothing.
