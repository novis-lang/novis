`#[Core\Route]` on a method gives it a path and an HTTP method. `#[Core\Access]` says who may
use it. A route needs both.

A path has fixed segments and captures. `{id}` matches one segment. `{id?}` matches one segment or
none, and must come last. `{path...}` matches all remaining segments. Each capture goes to
the parameter with the same name, converted to its type.

Novis collects every route into one table at compile time. `Core\Router::url` builds a
link from the name of a route. An unknown route name or a missing capture is a compile error.

The server matches each request to a route before your program runs. Your entry file calls
`Core\Router::dispatch`, which calls that route's method. A program can also read the match with
`Core\Request::route()`.

A parameter with `#[Core\Query]` gets its value from the query string, converted to its type. A bad
value is a `400`.

**The examples below** show a route and its link, the three captures, product lists, routes in
another file, calling a route method yourself, a route with no `#[Core\Access]`, and a bad query
value.
