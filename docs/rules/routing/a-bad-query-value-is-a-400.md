A `#[Query]` parameter's value converts to its declared type and arrives unqualified, exactly as a path
capture does — and fails differently. A path capture that does not convert means *this route is not the
one*: matching continues, and a miss is a `404`. A query value cannot select a route, so there is
nothing to continue to: the route matched and the input is bad, which is what `400` means. A required
key that is absent is a `400` for the same reason.

The divergence is the point rather than an inconsistency, and it is pinned as one: the two failures are
asserted side by side on one route, so a reader holding two adjacent lines of one signature knows they
fail two ways.

**A path capture at a class built from text is on the query value's side of that line**, and for the
query value's reason. It is not one of the conversions the matcher performs
(`rule:security/route-capture-is-laundered-by-its-type`): it matches on shape and converts where the
match crosses into the program, so by the time the class can refuse anything the route has already been
chosen and there is nothing to continue to. Its refusal is therefore a `400`, and what makes the two
kinds of capture differ is which of them the router itself reads.

**The query half is not shipped.** The declaration — the marker, its type list, its optional bit
(`rule:routing/a-query-parameter-is-declared-like-a-capture`) — compiles today; nothing in the tree yet
converts a query value into a bound parameter or answers `400` for one. `Core\Router\Match::params`
carries the path captures alone, and a program reads `Core\Request::query()` raw.
