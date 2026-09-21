---
summary: the route a request matched — its declared `name` and the captures its path filled, decided once at the door
keywords: route match, matched route, route parameters, path captures, Core\Request::route, named route, tainted capture
---

A `Core\Router\Match` is what `Core\Request::route()` answers (`null` when nothing in the route
table claimed this method and path); it is never constructed by hand. The server matches the
incoming request against the compiled table **once**, before any of the program runs, and this is
that result travelling on the request — so a handler never matches its own path a second time.
Matching is not dispatching: nothing here calls the annotated method, and the matched route's
handler, its declared verb and its access decision do not cross. The program calls the method
itself, with one `switch` on `name()`; [the attributes chapter](#lang-attributes) shows that
`switch`, and how the program sends the `404` and the `405`.

`name()` is the route's `#[Core\Route(name: …)]` as the program wrote it, or `null` for a route
that declares none; it is a plain `string`, because it is the unit's own literal. `params()` is
every capture the path filled, keyed by the parameter it binds, and `param()` is that array read at
one key — `null` for a name the route does not declare, including an optional `{name?}` the request
left off. A capture is `tainted string` where the route declared `string`, still percent-encoded,
and the `int`, `uint`, `decimal` or `Core\Uuid` the match already converted where it declared one of
those, so a handler never parses a segment the router has parsed already — and a segment that would
not convert did not match the route in the first place.

Reading it needs a request. Off the command line — and in a scheduled script, a job worker or a
test — there is none, and the reader refuses rather than answering `null`, because "no request
arrived" and "nothing matched" are different facts.

```nvs
<?nvs
try {
    Core\Request::route();
} catch (LogicError $why) {
    echo "no request here\n";
}
```
```output
no request here
```
