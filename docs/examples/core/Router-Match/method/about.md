Returns the HTTP method of the route that matched, as a `Core\Http\Method` case. It is the method
in the route's `method:`.

One handler can have two `#[Route]` attributes, for example one for `Get` and one for `Post` on
the same path. `is()` returns `true` for both routes, because both belong to the same handler.
`method()` tells them apart.

A `HEAD` request matches a `Get` route. For that request, the result is `Core\Http\Method::Get`.

The examples show the method of a match, a `HEAD` request, and one handler that shows a form for
`Get` and saves it for `Post`.
