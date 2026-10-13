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

**The path half is built; the query half is M16's.** The declaration — the marker, its type list, its
optional bit (`rule:routing/a-query-parameter-is-declared-like-a-capture`) — compiles, and both path
failures answer as above. Nothing yet binds a query value to a parameter or answers `400` for one: a
program reads `Core\Request::query()` and converts the value itself. M16 binds each `#[Query]`
parameter from the query string when a route method is called, and answers `400` for a value that
does not convert or a required key that is absent.
