`nvs code` prints information about the code of the project you are in, and it can change that code
for you.

`refs`, `calls`, `impls` and `types` show where a name is used, which methods call a method, and
which type a variable has. `unused` lists each declaration that nothing uses, and `delete` deletes one.

`rename` renames a name everywhere. `nvs code rename 'Shop\Cart' 'Shop\Billing\Cart'` moves a class
to another namespace. `rename --from <file>` applies a list of renames in one change. `move` moves a
file, and an `autoload` file gets the class name its new path gives. `move 'Shop\Cart::total'
'Shop\Order'` moves a method to another class. `extract` writes a new variable, method or interface.
`inline` replaces a variable or a call with its value. `signature` changes a method's parameters at
every call. `implement` writes the methods a class is missing, and `order` sorts class members.

Each command that changes files takes `--dry-run` and `--json`. If the new code would not compile,
the command changes nothing and prints why.

**Good to know:** write the name with its namespace, in single quotes. Without the quotes, the shell
removes the backslashes and the name matches nothing.
