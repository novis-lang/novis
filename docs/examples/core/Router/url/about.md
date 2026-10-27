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

If a path changes, you change it in the `#[Route]` only. Every link to that method follows it.

The examples show a link with one capture, values that become the query string, and the links of a
menu.
