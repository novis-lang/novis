Returns a new `Uri` with some parts of the address changed. You name the parts in a block, for example
`with({scheme: "https", port: null})`. The parts are `scheme`, `host`, `port`, `path`, `query` and
`fragment`. Each part you do not name stays the same. The `Uri` you call it on does not change.

`null` removes a part. This works for `port`, `query` and `fragment`. An empty `query` is different:
`query: ""` gives an address that ends with `?`.

You write each new part already encoded, the way it appears in the address. The user part
(`userInfo`) is always kept, and `with` cannot change it.

`with` throws a `RuntimeError` when the new address is not valid. A port above `65535` is one case. A
path without `/` at the start, next to a host, is another.

**The examples below** move a link to `https`, remove parts of an address, and turn an internal
address into a public one.
