Returns one value that a route captured from the path, found by its name. You write the name without
the braces: `parameter("id")` for `{id}`. The result is the same value that `parameters()` has under
that key.

The value is already percent-decoded, and it has the type of its handler parameter. A text
capture is `tainted`, because the visitor chose it.

The result is `null` when the route has no capture with that name. It is also `null` when an
optional capture such as `{page?}` is not in the path, so `??` can give a default.

Inside `Core\Router::dispatch`, the match also has each `#[Core\Query]` value under its parameter's
name. So a gate's `allows` method can read a query value. The match from `Core\Request::route()` has
no query values, and `parameter` returns `null` for them.

The examples show how to read one capture, a default for an optional capture, how to find the
record that the path names, and how a gate reads a query value.
