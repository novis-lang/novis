Checks whether the route that a path matched belongs to a method. You write the method the same
way as for `Core\Router::url`, without `()`: `$match->is(Shop::product)`. Nothing is called. The
result is `true` when the route belongs to that method, and `false` otherwise.

A method can have more than one `#[Route]`, for example a `GET` and a `POST` route with the same
path. `is` returns `true` for each of these routes.

You get a match from `Core\Request::route()` for the request that is being served, or from
`Core\Router::match` for a path that you choose.

The method can also be in a variable, for example an item of an `array<callable>` menu. A method
that you write in the call must have a `#[Route]`, or the program does not compile. A variable
whose method has no `#[Route]` throws a `RuntimeError`.

The examples check the route of a served request, use a method in a variable, and call the right
method for each route.
