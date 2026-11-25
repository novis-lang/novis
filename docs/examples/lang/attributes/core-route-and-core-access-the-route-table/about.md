`#[Core\Route]` on a method gives it a path and an HTTP method. `#[Core\Access]` says who may
use it. A route needs both.

A path has fixed segments and captures. `{id}` matches one segment. `{id?}` matches one segment or
none, and must come last. `{path...}` matches all remaining segments. Each capture goes to
the parameter with the same name, converted to its type.

Novis collects every route into one table at compile time. `Core\Router::url` builds a link to
the route of a method. You write the method without brackets, such as `Shop::show`. An unknown
method, a method with no route, or a missing capture is a compile error.

The server matches each request to a route before your program runs. Your entry file calls
`Core\Router::dispatch`, which calls that route's method. A program can also read the match with
`Core\Request::route()`.

A parameter with `#[Core\Query]` gets its value from the query string, converted to its type. A bad
value is a `400`, and the route method does not run. If a bad value should give a default instead,
declare the parameter as a `string` and convert it yourself. `($page as ?uint) ?? 1` gives `1` for
text that is not a whole number.

A parameter written `?uint $page = null` is optional. When the key is missing, the value is `null`.
When the key is there, its value converts as a `uint`, and a bad value is a `400`. This works for
every type that a query parameter can have, such as a class that implements `Parses`.

A `bool` query value is `1` for `true` and `0` for `false`. Any other text, such as `true` or `on`,
is a `400`. A checkbox in a form sends `on`, so give it `value="1"`.

**The examples below** show links, the three captures, routes in another file, calling a route
method yourself, a missing `#[Core\Access]`, a bad query value, a query value that may be missing,
and a `bool` query value.
