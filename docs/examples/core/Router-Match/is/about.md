Returns the name of the route that a path matched. The name is the text in the route's
`#[Route(name: …)]`. When the route has no name, the result is `null`.

You get a match from `Core\Request::route()` for the request that is being served, or from
`Core\Router::match` for a path that you choose. `Core\Router::url` builds a link from the same
name, so one name finds a route in both directions.

The name comes from your program and not from the request, so it is not `tainted`. You can print
it or use it in a `switch` with no extra check.

The examples show the name of a served request, a route with no name, and a `switch` that calls
the right method for each route name.
