Novis has no global functions, global constants or global variables. This section lists what to
write for each PHP form.

A function is a static method of a class. A constant is a class constant with a visibility and a
type, such as `public const int X = 1;`. It replaces a top-level `const` and `define`. A variable is
declared once with its type, as `int $x = 1;` or `var $x = 1;`.

`global`, `$$name`, `eval`, `extract` and `compact` do not exist. Pass a value as a parameter, use a
static property, or use an array whose keys are the names. A `static` variable inside a function
becomes a `private static` property. Novis fills no variable for you, so `$_GET`, `$_POST` and
`$argv` do not exist. A program reads the data of a request through `Core` classes.

**Good to know:** `unset` removes an entry of an array and nothing else. To clear a variable of type
`?T`, assign `null`.
