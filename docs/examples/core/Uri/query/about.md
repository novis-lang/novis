Returns the query of an address, which is the text after the `?`. For
`https://shop.example.com/search?q=shoes&page=2#top` the result is `q=shoes&page=2`.

The query is returned as it was written. Escapes such as `%20` and `+` stay in the text. To read one
value from it, use `queryParameter()`. To read all values into an array, use `Core\Uri::parseQuery`.

The result is `null` when the address has no `?`. It is `""` when the `?` has nothing after it. The
query ends at the first `#`, so the fragment is never part of it.

**The examples below** read the query of a link, show an address with no query and one with an empty
query, and pass the query of a request on to another server.
