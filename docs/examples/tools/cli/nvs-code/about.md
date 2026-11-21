`nvs code` answers questions about the project you are in: where a name is declared, where it is
used, who calls a method, and which type a variable has. Only `rename`, `move` and `order` change
files.

`nvs code refs 'Shop\Cart::addItem'` prints every place the method is written, the declaration
first. `nvs code calls` and `nvs code reach` take one or more methods. With several, each method
gets a line with its name, and its answer is indented under it.

`nvs code order --check` prints each class whose members are not in the standard order, and exits
with an error, so a CI job can run it.

The editor and `nvs code` share one index file. A call checks the content of every file first and
analyses only the files that changed. So the answer is always current, and it is fast.

**Good to know:** write the name with its namespace, in single quotes. Without the quotes, the shell
removes the backslashes and the name matches nothing.
