Some ways of writing code that PHP allows do not exist in Novis. Writing one is a compile error, and
the message names what to write instead.

The list is `eval`, `extract`, `settype`, `compact` and PHP's other free functions, variable
variables such as `$$name` and `${$expr}`, `list(...)`, casts such as `(int)$x`, the `@` sign,
`=&`, backticks, `die`, `include` and `require_once`, `yield` read as a value, and `self`, `static`
or `parent` written outside a class. Each one has a replacement: a method under `Core` for a PHP
function, `as` for a conversion, `[...]` for taking a list apart, `require` for reading another
file, and `exit` for ending a program.

**Good to know:** you meet all of these when you compile, never while the program is running.

**The examples below** show the three replacements you reach for most often: a `Core` method in
place of a PHP function, `as` in place of a cast, and `[...]` in place of `list(...)`.
