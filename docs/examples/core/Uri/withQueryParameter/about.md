Returns a new `Uri` with one query parameter set to a new value. For `/search?q=shoes&page=2`,
`withQueryParameter("page", 3)` returns `/search?q=shoes&page=3`. The `Uri` you call it on does not
change.

When the name is not in the query yet, the pair is added at the end. The value is encoded for you, so
a space becomes `+` and `&` becomes `%26`. An array value is written with brackets, the same way
`Core\Uri::buildQuery` writes it.

A `null` value removes the parameter. When you remove the last parameter, the address has no `?` at
all.

**Good to know:** the whole query is written again. The other pairs keep their values, but the way
they are encoded can change. For example, `%20` becomes `+`.

**The examples below** change a page number, remove a parameter, and build the links of a page list.
