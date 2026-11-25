A `#[Query]` parameter's value converts to its declared type and arrives unqualified, exactly as a path
capture does, and fails differently. A path capture that the matcher cannot convert means *this route is
not the one*: matching continues, and a miss is a `404`. A query value takes no part in choosing a route,
so there is nothing to continue to: the route matched and the input is bad, which is what `400` means. A
required key that is absent is a `400` for the same reason, and the handler's own body never runs. A
`bool` value is `1` or `0`, and any other text, an empty value or a bare key is a `400`
(`rule:security/route-capture-is-laundered-by-its-type`).

**A path capture at a class built from text is on the query value's side of that line**, and for the
query value's reason. That class is not one of the conversions the matcher performs
(`rule:security/route-capture-is-laundered-by-its-type`): the matcher matches its segment on shape and
the class converts it where the match crosses into the program, so by the time the class can refuse
anything the route has already been chosen and there is nothing to continue to. Its refusal is therefore
a `400`. What makes the two kinds of capture differ is which of them the router itself reads. Running an
implementor's `parse` in the matcher would put a body that can loop, allocate and throw over every
request URL, including the ones that match nothing.

The divergence is the point rather than an inconsistency, and it is pinned as one: one case asserts a
path capture's `404` and a text-built capture's `400` side by side, so a reader holding two adjacent
lines of one signature knows they fail two ways.

**A key written twice is a `400` too**, for a parameter that takes one value: neither value is picked
(`rule:errors/ambiguous-input-refused`). So is a bracketed key such as `page[]=1`, which builds a list.

`Core\Router::dispatch` binds each `#[Query]` parameter when it calls a route method, after the match
and before `allows`. It reads the query string through the parser `Core\Request::queryParameter()` uses, once per
request and only for a route that declares a `#[Query]` parameter, so a route without one costs nothing
more. An absent optional key gives the declared default. Every failure above is `refused` with
`BadCapture` and the match, which then holds the captures alone.

**Each bound value is written into the match before `allows`**, under its parameter's name, so
`Core\Router\Match::parameter('order')` reads a `#[Query] Order $order` exactly as it reads an
`{order}` capture, and the gate checks every input the route method will receive. An absent
`?T $x = null` key is left out, as an absent optional capture is, so `parameter()` reads `null`; an
absent key with another default holds that default. An enum value stays the plain integer, as an enum
capture does. **`Core\Request::route()` reads no query value**: it is the door's match, built before
any query string is read, so `parameter()` is `null` for a `#[Query]` name there. A program that
dispatches by hand reads the query string itself. **What it spends:** one array entry per bound value
per matched request, a second reference to the value `dispatch` already holds, so nothing is copied.

**Declined:**
- **A lenient mode, where a value that does not convert gives the default.** A stale link such as
  `?page=abc` would then show page 1 with no sign that its input was wrong, and the same silent
  fallback would hide a typo or a probe. A route that wants the fallback declares the parameter
  `string` and converts it itself — `($page as ?uint) ?? 1` — so the choice is written where it is
  made.
- **Leaving the query values off the match.** `allows` would then never see a record a query value's
  `parse` loaded, and the route method would receive it unchecked — the hole the gate exists to close.
