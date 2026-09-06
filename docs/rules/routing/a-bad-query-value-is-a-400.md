A `#[Query]` parameter's value converts to its declared type and arrives unqualified, exactly as a path
capture does — and fails differently. A path capture that does not convert means *this route is not the
one*: matching continues, and a miss is a `404`. A query value cannot select a route, so there is
nothing to continue to: the route matched and the input is bad, which is what `400` means. A required
key that is absent is a `400` for the same reason.

The divergence is the point rather than an inconsistency, and it is pinned as one: the two failures are
asserted side by side on one route, so a reader holding two adjacent lines of one signature knows they
fail two ways.

**Not shipped.** The declaration half — the marker, its type list, its optional bit
(`rule:routing/a-query-parameter-is-declared-like-a-capture`) — compiles today; nothing in the tree yet
converts a query value into a bound parameter or answers `400` for one. `Core\Router\Match::params`
carries the path captures alone, and a program reads `Core\Request::query()` raw.
