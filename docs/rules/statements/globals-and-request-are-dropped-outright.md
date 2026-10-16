`$GLOBALS` and `$_REQUEST` are dropped with no replacement of any kind: no `Core` class, no method, no
later addition under a different name.

`$GLOBALS` is not data the host hands a script — it is a reflective view onto the script's *own* variable
table, keyed by name and mutable in both directions. A top-level variable is a local of the script's own
frame (`rule:statements/storage-that-outlives-a-call`), so there is nothing left for it to expose that a
`static` property, a constant, an object property or a parameter does not already reach the declared way. A
replacement would have to enumerate a frame's locals reflectively, which nothing else here does, or become
a second informally-scoped static bag.

`$_REQUEST`'s only job is merging three sources whose read-site names — `Core\Request::query()`, `::post()`
and `::cookie()` — already say which one a value came from. A merged accessor would either fix an order,
re-deriving the `request_order` footgun, or take the order as an argument, at which point it is no shorter
than naming the source.
