Where two routes of one verb both match a path, the one with more fixed segments wins, segment by
segment: a fixed segment beats a `{name}`, which beats a `{name?}`, which beats a `{name...}`. `/users/new` and
`/users/{id}` coexist with no ordering rule to remember, and moving a declaration between files
cannot change which route answers. That is what makes the table order-independent at all, and it is
why two routes with the same verb and the same shape are refused rather than ordered
(`rule:routing/routes-are-compiled-not-registered`).

A failed conversion is not a match: `/users/abc` against `show(uint $id)` passes on to the next
candidate, and is a `404` only if nothing else claims the path
(`rule:security/route-capture-is-laundered-by-its-type`). A `{name?}` is one node marked terminal, so
`/posts` and `/posts/2` each cost one walk rather than two.

The model is the radix-trie rule `matchit` implements — the rule is the precedence, not the data
structure, and a matcher is free to compute the same answer by ranking rows.
