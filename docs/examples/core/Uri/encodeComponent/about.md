Escapes a text so that you can put it into one part of a link, such as a path segment, a fragment or
a query value.

Some characters have a meaning in a link. `/` starts a new part of the path, `?` starts the query
and `&` starts a new query pair. `Core\Uri::encodeComponent` writes each of these as a `%` and two
hex digits. It does the same for every other character except letters, digits and `-_.~`.
`Core\Uri::encodeComponent("red shoes & bags")` returns `red%20shoes%20%26%20bags`. A character such as
`é` is two bytes, so it becomes two escapes.

A text that is exactly `.` or `..` throws a `LogicError`. In a path, such a segment changes which
resource the link names. For a query value, use `Core\Uri::withQueryParameter` or
`Core\Uri::buildQuery`.

`Core\Uri::decodeComponent` reads the text back. For a value in a form, use
`Core\Uri::encodeFormValue`, which writes a space as `+`.

**The examples below** link to a heading, show which characters are escaped, and build download
links for uploaded files.
