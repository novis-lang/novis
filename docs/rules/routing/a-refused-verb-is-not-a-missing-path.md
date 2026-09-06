`Core\Router::methodsFor(tainted string $path): array<Http\Method>` tells a missing path from a refused
verb. An empty array means no route claims the path at all — `404`. A non-empty one means the path
exists under other verbs — `405`, and the list is the `Allow:` header RFC 9110 requires beside it. Each
verb appears once, as a `Core\Http\Method` case, in declaration order.

It is asked only after `Core\Request::route()` or `match` has answered `null`, so a served request pays
nothing for it, and `match` keeps its two-state return rather than growing a verb list on the success
path where nobody reads it. A program with no `#[Route]` answers the empty array, because "no route
claims this path" is exactly true of it.

Where an optional trailing capture makes a node terminal
(`rule:routing/a-trailing-segment-may-be-absent`), both forms report the same verbs:
`methodsFor("/posts")` on a table declaring `/posts/{page?}` returns that route's verbs, so the shorter
form cannot `404` while the longer one `405`s. A plain `OPTIONS` is answerable by an application from
this list; the server still answers none on its behalf, because which to answer is a convention.
