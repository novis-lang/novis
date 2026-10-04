A `$params` key handed to `Core\Router::url` or `urlAbsolute` that names no capture becomes a
percent-encoded query string, spelled exactly as `Core\Uri::buildQuery` spells it:
`url("posts.index", {page: 3, sort: "desc"})` on `/posts/{page?}` folds to `/posts/3?sort=desc`. The
link is still the launderer's (`rule:security/launderers-are-sink-named`) — a paginated link has a
laundered spelling, where `url(…) . "?page=" . $n` is the concatenation the launderer exists to prevent
and re-taints the result.

The refusal half is the same sentence. A key written in the `$params` array literal that is neither a capture nor one of the route's
declared `#[Query]` parameters (`rule:routing/a-query-parameter-is-declared-like-a-capture`) is a
**compile error**, in the same shape as the unknown-name and missing-capture errors, so a typo cannot
silently ship as a query parameter. The declared type says what a query *value* arrives at; it does not
constrain what a link may be handed, except that a literal outside a closed set is refused
(`rule:routing/a-capture-narrows-to-a-closed-set`).
