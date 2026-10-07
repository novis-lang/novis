Chooses the best of your offered locales for an `Accept-Language` header.

`Icu::negotiate` takes the header's value, the list of locales you offer, and a default. It returns
the offered locale with the highest weight in the header. A regional tag such as `"de-AT"` matches an
offered `"de"`. An empty, malformed or unmatched header returns the default, and does not throw.

`Icu::negotiate` throws a `LogicError` when a tag in the offered list is not valid.

**Good to know:** most programs do not call `Icu::negotiate` directly. `Locale::negotiate` is the
same function.
