Returns every value that a route captured from the path, as an array. Each key is the name in the
route's path, such as `id` for `{id}`. The values are in the order of the path.

Each value is already percent-decoded, so `%20` in the path is a space. Each value also has the
type of its handler parameter: `{id}` read by `uint $id` is a number. A text capture is `tainted`.
That means it came from the visitor, so you must check it before you use it.

An optional capture that the path does not have is not in the array. A route with no captures
returns an empty array.

Inside `Core\Router::dispatch`, the array also has each `#[Core\Query]` value, under its parameter's
name. A query value with a default other than `null` is always in the array. A query value with the
default `null` is left out when the request does not have it. The match from
`Core\Request::route()` or `Core\Router::match()` has only the path captures.

The examples read the captures of a path, routes with no captures and a log line. The last one
shows what a gate sees.
