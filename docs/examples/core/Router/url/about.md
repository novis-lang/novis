Builds the link to one of your routes. You give `Core\Router::url` the method that has the
`#[Route]`, and an array of values. You write the method without `()`, such as `Articles::show`.
The link does not call the method. Each value fills the capture with the same name in the route's
path, and the other values become the query string. The result is a string such as
`/articles/42?page=2`. A route with no captures needs no array: `Core\Router::url(Shop::home)`.

Every value is percent-encoded. A value that contains `/`, `?` or `&` stays inside its own part of
the link, so a visitor's text cannot change where the link goes. If the method has no `#[Route]`,
the program does not compile.

The method can also be a variable, for example an item of an `array<callable>` menu. For an
instance method, write `$shop->order(...)`. A variable whose method has no `#[Route]` throws a
`RuntimeError` when the call runs.

A `bool` value is written `1` or `0`. A date or time value is written as its ISO text, such as
`2026-03-09`. An object of a `Parses` class is written as the text of its `format` method. A query
value that is `null` is left out of the link. A value whose type its parameter cannot take does not
compile, such as a date for a `uint` capture.

If a path changes, you change it in the `#[Route]` only. Every link to that method follows it.

The examples show a link with one capture, query values, the links of a menu, a link to another
module, and a `Parses` capture.
