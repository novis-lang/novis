Escapes a text so that you can put it into one part of a link, such as a path segment, a fragment or
a query value.

A link can only contain some characters, and some of them have a meaning: `/` starts a new part of
the path, `?` starts the query and `&` starts a new query pair. `Core\Uri::encodeComponent` writes
every other character as a `%` and two hex digits. `Core\Uri::encodeComponent("red shoes & bags")`
returns `red%20shoes%20%26%20bags`. Letters, digits and `-_.~` are not changed. A character such as
`é` is two bytes, so it becomes two escapes.

`Core\Uri::decodeComponent` reads the text back. For a value in a form, use
`Core\Uri::encodeFormValue`, which writes a space as `+`.

**The examples below** link to a heading, show which characters are escaped, and build download
links for uploaded files.
