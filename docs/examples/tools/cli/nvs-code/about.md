`nvs code` prints information about the code of the project you are in. It shows where a name is
declared, where it is used, which methods call a method, and which type a variable has. Only `rename`, `move` and `order` change
files.

`nvs code refs 'Shop\Cart::addItem'` prints every place the method is written, the declaration
first. `nvs code calls` and `nvs code reach` take one or more methods. With several, each method
gets a line with its name, and its own lines are indented under it.

`nvs code order --check` prints each class whose constants, properties and methods are not in the
standard order. Then it exits with status `1`, so a CI job can run it.

The editor and `nvs code` share one index file. It lists every name in the project and every place
it is used. Each call compares the content of every file with the index file. Then it analyses only
the files that changed. So the result is always current, and the call is fast.

**Good to know:** write the name with its namespace, in single quotes. Without the quotes, the shell
removes the backslashes and the name matches nothing.
