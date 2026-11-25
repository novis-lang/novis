Returns one value that a route captured from the path, found by its name. You write the name without
the braces: `parameter("id")` for `{id}`. The result is the same value that `parameters()` has under
that key.

The value is already percent-decoded, and it has the type of its handler parameter. A text
capture is `tainted`. That means it came from the visitor, so you must check it before you use it.

The result is `null` when the route has no capture with that name. It is also `null` when an
optional capture such as `{page?}` is not in the path, so `??` can give a default.

Inside `Core\Router::dispatch`, the match also has each `#[Core\Query]` value under its parameter's
name. So a gate's `allows` method can read a query value. The match from `Core\Request::route()` or
`Core\Router::match()` has no query values, and `parameter` returns `null` for them.

The examples read one capture and give an optional capture a default. They also find the record
that the path names, and read a query value in a gate.
