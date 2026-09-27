Builds the link to one of your routes. You give `Core\Router::url` the name of a `#[Route]` and an
array of values. Each value fills the capture with the same name in the route's path, and the other
values become the query string. The result is a string such as `/articles/42?page=2`.

Every value is percent-encoded. A value that contains `/`, `?` or `&` stays inside its own part of
the link, so a visitor's text cannot change where the link goes. The route name must be a literal
string. If the name or a key does not exist, the program does not compile.

If a path changes, you change it in the `#[Route]` only. Every link that uses the name follows it.

The examples show a link with one capture, values that become the query string, and the links of a
menu.
