Writes an array of names and values as a query string, the part of a link after the `?`.

`Core\Uri::buildQuery(["q" => "red shoes", "page" => 2])` returns `q=red+shoes&page=2`. Each name and
value is escaped, so a space becomes `+` and a character such as `&` or `=` cannot start a new pair. A
value can be a list or another array. Its items are written under their whole path, such as
`sizes[0]=M`. `true` is written as `1` and `false` as `0`. A `null` value is left out.

`Core\Uri::parseQuery` reads the text back to the same names and values.

**The examples below** build a search link, write lists and nested values, and build the link to the
next page of a list.
