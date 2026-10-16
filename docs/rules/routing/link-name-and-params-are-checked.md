`Core\Router::url(string $name, array<string, mixed> $params): string` reverses the table by name,
and the compiler resolves the call against the whole program's table after every file has been
walked — the route may be declared in a file the scan reaches later.

Two failures are compile errors, reported at the argument: **a `$name` given as a string literal that
no `#[Route]` declares**, and **a `$params` array literal that does not cover the route's captures** —
every `{name}` and `{name...}` the path writes needs a key of that name, and only a `{name?}` may be
left out, whose whole segment is then dropped. A key in that array literal that is neither a capture nor one of the route's declared
`#[Query]` parameters is refused in the same shape, so a typo cannot silently become a query
parameter. A **computed** `$name` throws instead, and a computed `$params` is not checked at all —
there are no keys to read.

The result is a laundered URL path (`rule:security/launderers-are-sink-named`): each substituted
value is percent-encoded into its own segment under RFC 3986's component rules, so a `tainted`
parameter produces a plain `string` that is safe *as a path* and for nothing else. `urlAbsolute` is
the same link with a configured origin in front, and refuses exactly the same names. A route with no
`name` cannot be linked to at all (`rule:routing/route-attribute`), and the path a link is built on is
the mount-relative one (`rule:routing/link-carries-the-mount-prefix`).
