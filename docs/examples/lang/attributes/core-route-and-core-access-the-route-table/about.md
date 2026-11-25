`#[Core\Route]` on a method gives it a path and an HTTP method. `#[Core\Access]` says who may
use it. A route needs both.

A path has fixed segments and captures. `{id}` matches one segment, `{id?}` one or none, and
`{path...}` all remaining segments. Each capture goes to the parameter with the same name, converted
to its type. A parameter with `#[Core\Query]` gets its value from the query string. A bad query
value is a `400`, and the route method does not run.

`Core\Router::url` builds a link to the route of a method, such as `Shop::show`. Your entry file calls `Core\Router::dispatch`, which calls the
method of the route that matches.

A `Core\Time\Date`, `TimeOfDay` or `Instant` can be a capture or a query value. A `DateTime` can only be a query value.
Its zone name, such as `Europe/Vienna`, has a `/`, and in a path a `/` would split the value.

**The examples below** show links, captures, query values that may be missing, `bool` and time
values, and a missing `#[Core\Access]`.
