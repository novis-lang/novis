---
summary: the route a request matched — the method it belongs to, the captures its path filled, its verb and its access decision, decided once at the door
keywords: route match, matched route, route parameters, path captures, Core\Request::route, route method, tainted capture, access decision, #[Core\Access], allow, authorization, dispatch
---

A `Core\Router\Match` is what `Core\Request::route()` answers (`null` when nothing in the route
table claimed this method and path); it is never constructed by hand. The server matches the
incoming request against the compiled table **once**, before any of the program runs, and this is
that result travelling on the request — so a handler never matches its own path a second time.
Matching is not dispatching: nothing here calls the annotated method, and a match has nothing that
can be called. The program calls the method itself, after it checks which method matched with `is()`;
[the attributes chapter](#lang-attributes) shows that check, and how the program sends the `404`
and the `405`.

`is(Shop::show)` returns `true` when the route belongs to that method. You write the method the way
`Core\Router::url` takes it: `Class::method` with no `(...)`, for a static or an instance method. A
method from a variable, such as an `array<callable>` or `$shop->show(...)`, is checked while the
program runs, and `is` throws an error when that method has no route. `params()` is
every capture the path filled, keyed by the parameter it binds, and `param()` is that array read at
one key — `null` for a name the route does not declare, including an optional `{name?}` the request
left off. A capture is `tainted string` where the route declared `string`, still percent-encoded,
and the `int`, `uint`, `decimal` or `Core\Uuid` the match already converted where it declared one of
those, so a handler never parses a segment the router has parsed already — and a segment that would
not convert did not match the route in the first place.

`access()` is the route's `#[Core\Access(allow: …)]` as a `string`: the full name of the constant,
such as `Core\Audience::Public` or `App\Role::Admin`. The server does not enforce it. A program
that calls route methods checks it once, before the `switch`, so the check also covers a route
added later. Treat `null` as access denied. `method()` is the verb the matched `#[Core\Route]`
declares, which tells apart the routes of one method that serves several verbs.

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
