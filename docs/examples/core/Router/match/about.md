Finds the route that an HTTP method and a path belong to. `Core\Router::match` checks them against
every `#[Route]` in your program and returns a `Core\Router\Match`. The match has the route's name
and its captures. It does not run the route's handler.

The captures are decoded, and each one has the type of its handler parameter. For example,
`{id}` is a number when the handler takes `uint $id`. If no route has that method and that path,
the result is `null`. A capture whose escapes are not valid UTF-8 throws a `RuntimeError`.

A request that is being served already has its match: use `Core\Request::route()` for that.
`Core\Router::match` is for a path that your program chose, for example a link in a menu.

The examples show a match and its captures, a path that no route matches, and a check of every
link in a menu.
