**A directive is `System` when changing it from inside a request would affect something other than
that request.** That is the whole test, and it covers the `[[extension]]` entries and their hash
pins, `cache.dir`, `opcache.validate` and its rate cap, the per-app blocks, `[limits.hard]` and
`[mode] ceiling` themselves, every `[[schedule]]` key, `[deferred] max_concurrent` and both
observability blocks. A directive being `System` is what makes it a limit; there is no separate notion
of a "locked" value.

A response header is the counter-example and is ordinary `Runtime`: a request may set any HTTP policy
directive for itself, because it could already write the header directly and the change dies with the
request.

`System` is **not** a synonym for "read once at boot". Most `System` directives reload: whether a
change needs a new snapshot or a restart is `rule:config/reloadability-is-its-own-field`'s question,
held in a field of its own, and the registry's census test fails if either field is ever derived from
the other. With that split, `System` means one thing again — a request may not set it.
