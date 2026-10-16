Returns one value from the query of an address, by its name. For
`/search?q=red+shoes&page=2`, `queryParameter("q")` returns `red shoes`.

The value is decoded, so `+` becomes a space and `%2F` becomes `/`. It is `bytes`, because an escape
can give any byte. Use `as string` to get text. When a name appears twice, the last value is kept.

Brackets in a name build arrays, the same way `Core\Uri::parseQuery` reads them. For `size[]=M&size[]=L`,
`queryParameter("size")` returns a list with two values.

The result is `null` when the name is not in the query, and also when the address has no query.
Use `query()` to tell these two apart.

**Good to know:** each call reads the whole query again. To read many values, call
`Core\Uri::parseQuery` once.

**The examples below** read one value, read a list and a group of values, and read the page to return
to after a login.
