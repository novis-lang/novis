Reads the text of an address and returns a `Uri`. An address such as
`https://shop.example.com:8443/cart?sort=price#total` has up to seven parts: the scheme, the user,
the host, the port, the path, the query and the fragment. Each part has its own method, such as
`host()` or `query()`.

Every part is returned as it was written. Escapes such as `%20` stay in the text, and upper-case
letters stay upper case. A relative link such as `/docs/start` is allowed, and its scheme and host
are `null`.

`Core\Uri::parse` throws a `RuntimeError` when the text is not an address. A space, a control
character or a `%` without two hex digits all cause this. `Core\Uri::tryParse` returns `null`
instead.

**The examples below** print each part of an address, show a relative link and text that is not an
address, and read a database setting.
