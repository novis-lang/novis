Builds the link to one of your routes. You give `Core\Router::url` the method that has the
`#[Route]`, written without `()`, such as `Articles::show`, and an array of values. Each value fills
the capture with the same name in the path. The other values become the query string. The result is
a string such as `/articles/42?page=2`.

Every value is percent-encoded, so a visitor's text cannot change where the link goes. A method
written in the call must have a `#[Route]`, or the program does not compile.

The method can also be a variable, such as an item of an `array<callable>` menu. For an instance
method, write `$shop->order(...)`. When the variable is not a method with a `#[Route]`, `url`
throws a `RuntimeError`.

A `bool` is written `1` or `0`, and a date as its ISO text. A `Parses` object is written with its
`format` method. A `null` query value is left out. A value of the wrong type does not compile.

When a path changes, you change only the `#[Route]`, and every link follows it.

The examples show captures, query values, a menu and a link to another module.
