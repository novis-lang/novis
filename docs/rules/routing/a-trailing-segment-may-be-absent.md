`{name?}` captures one whole segment or none. It is permitted only in the last position, at most once,
and never in the same path as a `{name...}`. **The method parameter it binds must have a default** —
that is what makes the absent case well-typed rather than nullable by accident — and a `{name?}` whose
parameter has none is a compile error naming both sites, as is one in any other position.

Precedence slots it below `{name}` and above `{name...}`, so the structural rule extends with no
ordering to remember; in the trie it is one node marked terminal, costing a static hit's walk rather
than a second one. `/posts` matches `/posts/{page?}` with the parameter's default and `/posts/3` with
`3`. **`/posts/` matches neither**: an empty final segment is not an absent one, and repairing the
difference is what `rule:errors/ambiguous-input-refused` refuses.

It replaces the two-attribute spelling for one endpoint, which is legal but carries two paths — so the
two forms could not share a name, one of them lost reverse-URL generation, and metrics saw one endpoint
as two series. A link built with the optional capture left out of `$params` simply drops the segment.
