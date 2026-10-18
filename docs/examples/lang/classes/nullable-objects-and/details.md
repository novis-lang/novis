`->` on a value that may be `null` is the error `E0459`. This is true for a property and for a
method call.

The result of `?->` can be `null`. `$box?->count` on an `int` property is a `?int`. Assigning it to
an `int` variable is the error `E0401`. Write `$box?->count ?? 0` to get an `int`.

Each `?->` covers one step of a chain. In `$box?->inner()->label`, the result of `?->inner()` can be
`null`, so the `->` after it is `E0459`. This is true even when `inner()` returns a `Box`. Write
`?->` for every step after the first one: `$box?->inner()?->label`.

`$box?->save();` can call a `void` method as a statement. When `$box` is `null`, nothing runs.
`isset($box?->count)` is `false` when `$box` is `null`.
