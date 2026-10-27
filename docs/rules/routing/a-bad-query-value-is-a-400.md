A `#[Query]` parameter's value converts to its declared type and arrives unqualified, exactly as a path
capture does, and fails differently. A path capture that the matcher cannot convert means *this route is
not the one*: matching continues, and a miss is a `404`. A query value takes no part in choosing a route,
so there is nothing to continue to: the route matched and the input is bad, which is what `400` means. A
required key that is absent is a `400` for the same reason, and the handler's own body never runs.

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
and before `allows`. It reads the query string through the parser `Core\Request::query()` uses, once per
request and only for a route that declares a `#[Query]` parameter, so a route without one costs nothing
more. An absent optional key gives the declared default. Every failure above is `refused` with
`BadCapture` and the match. The proofs and the shipped status are goal `query-binding`'s.
