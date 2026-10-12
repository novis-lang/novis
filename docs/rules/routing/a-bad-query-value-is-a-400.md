A path capture that the matcher cannot convert means *this route is not the one*: matching continues,
and a miss is a `404`. A path capture at a class built from text fails differently, and the failure is a
`400`.

That class is not one of the conversions the matcher performs
(`rule:security/route-capture-is-laundered-by-its-type`): the matcher matches its segment on shape and
the class converts it where the match crosses into the program, so by the time the class can refuse
anything the route has already been chosen and there is nothing to continue to. The route matched and
the input is bad, which is what `400` means, and the handler's own body never runs. What makes the two
kinds of capture differ is which of them the router itself reads. Running an implementor's `parse` in
the matcher would put a body that can loop, allocate and throw over every request URL, including the
ones that match nothing.

The divergence is the point rather than an inconsistency, and it is pinned as one: one case asserts
both failures side by side, so a reader who sees two captures written the same way knows they fail two
ways.

A query value takes no part in choosing a route and is not bound to a parameter: a program reads
`Core\Request::query()` and converts the value itself.

**Declined:**
- **A `#[Query]` parameter bound from the query string and converted to its declared type, with a
  `400` for a value that does not convert.** A program reads `Core\Request::query()` and converts each
  value itself, so the request has one way to read a query value.
