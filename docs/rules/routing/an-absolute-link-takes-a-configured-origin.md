`Core\Router::urlAbsolute(string $name, array<string, mixed> $params): string` is `Core\Router::url`
with the configured origin in front — the same laundered path, the same query-string rule
(`rule:routing/a-leftover-link-key-is-a-query-string`), the same compile errors for an unknown name or
an uncovered capture — joined with exactly one `/` whatever the origin's trailing slash.

**The origin is configuration, never inference.** It is resolved before the program runs, out of
`[[app]] origin`, and is never derived from `Host` or `X-Forwarded-Host`: deriving an origin from a
header is host-header injection, and an emailed password-reset link is where it lands. A unit that
resolves no origin throws from `urlAbsolute` rather than answering a link with an empty authority in it —
`url` still answers for the same route, because a path needs no origin.

The two members are two names on purpose. One `url()` whose absoluteness depended on configuration would
mean two different things at one call site depending on a file the reader is not looking at, and would
break the property that one compiled table serves at any mount prefix. Where the origin comes from when
one binary serves several hosts is `rule:routing/an-origin-is-per-mount-and-checked-at-boot`.
